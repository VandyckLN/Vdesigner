//! Raster to SVG tracing, built on `visioncortex`.

use std::collections::HashSet;
use std::panic::{catch_unwind, AssertUnwindSafe};

use image::{DynamicImage, RgbaImage};
use serde::{Deserialize, Serialize};
use visioncortex::color_clusters::{KeyingAction, Runner, RunnerConfig, HIERARCHICAL_MAX};
use visioncortex::{
    Color, ColorImage, CompoundPath, CompoundPathElement, PathSimplifyMode, PointF64, PointI32,
};

use crate::error::CoreError;
use crate::resize::{resize, FitMode, ResizeSpec};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TraceMode {
    Mono,
    Color,
    /// Faithful photo: a large palette, background always kept.
    Photo,
}

/// Photo palette size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TraceDetail {
    #[default]
    Normal,
    High,
    Max,
}

impl TraceDetail {
    pub fn colors(self) -> usize {
        match self {
            TraceDetail::Normal => 64,
            TraceDetail::High => 128,
            TraceDetail::Max => 255,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceSpec {
    pub mode: TraceMode,
    /// 2 to 16. Validated in Color and Mono, used only by Color; Photo ignores it.
    pub colors: u8,
    /// Color only: the colour that dominates the border becomes transparent.
    pub remove_background: bool,
    /// Photo only.
    #[serde(default)]
    pub detail: TraceDetail,
}

impl Default for TraceSpec {
    fn default() -> Self {
        Self {
            mode: TraceMode::Color,
            colors: 8,
            remove_background: true,
            detail: TraceDetail::Normal,
        }
    }
}

/// Below this, the small-image path upscales to it.
pub const TRACE_MIN_SIDE: u32 = 512;
/// Above this, the image is reduced to it. Also caps how far a job enlarges its resize steps.
pub const TRACE_MAX_SIDE: u32 = 4096;

const OPAQUE: u8 = 128;
const SAMPLE_TARGET: usize = 65536;
const LLOYD_ITERATIONS: usize = 8;
const MONO_MIN_AREA: usize = 16;

// Curve parameters, shared by both modes.
const CORNER_DEGREES: f64 = 60.0;
const SEGMENT_LENGTH: f64 = 4.0;
const MAX_ITERATIONS: usize = 10;
const SPLICE_DEGREES: f64 = 45.0;

/// A pixel's palette index, or `None` when it is transparent.
type Label = Option<u8>;

/// `size` is what `width`/`height` say; the drawing itself is computed at the
/// working resolution and the `viewBox` carries that.
pub fn trace_svg(
    img: &DynamicImage,
    spec: &TraceSpec,
    size: (u32, u32),
    cancel: &dyn Fn() -> bool,
) -> Result<Vec<u8>, CoreError> {
    if spec.mode != TraceMode::Photo && !(2..=16).contains(&spec.colors) {
        return Err(CoreError::InvalidParameter(format!(
            "cores deve ficar entre 2 e 16, recebido {}",
            spec.colors
        )));
    }
    let source = img.to_rgba8();
    let body = match spec.mode {
        TraceMode::Color => trace_color(
            &source,
            spec.colors as usize,
            spec.remove_background,
            cancel,
        )?,
        TraceMode::Photo => trace_color(&source, spec.detail.colors(), false, cancel)?,
        TraceMode::Mono => trace_mono(&source, cancel)?,
    };
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">\n",
        size.0, size.1, body.width, body.height
    );
    for (d, fill) in &body.paths {
        out.push_str(&format!("<path d=\"{d}\" fill=\"{fill}\"/>\n"));
    }
    out.push_str("</svg>\n");
    Ok(out.into_bytes())
}

struct Traced {
    width: u32,
    height: u32,
    /// `(d, fill)` pairs, in drawing order.
    paths: Vec<(String, String)>,
}

fn longest_side(img: &RgbaImage) -> u32 {
    img.width().max(img.height())
}

/// Resamples so the longest side becomes `target`, on a premultiplied copy so
/// transparent pixels do not bleed their RGB into the edges.
fn resample(img: &RgbaImage, target: u32) -> Result<RgbaImage, CoreError> {
    let scale = target as f64 / longest_side(img) as f64;
    let side = |v: u32| ((v as f64 * scale).round() as u32).max(1);
    let mut pre = img.clone();
    for p in pre.pixels_mut() {
        let a = p[3] as u32;
        for ch in 0..3 {
            p[ch] = ((p[ch] as u32 * a + 127) / 255) as u8;
        }
    }
    let spec = ResizeSpec {
        width: Some(side(img.width())),
        height: Some(side(img.height())),
        fit: FitMode::Stretch,
        pad_color: [0, 0, 0, 0],
    };
    let mut out = resize(&DynamicImage::ImageRgba8(pre), &spec)?.to_rgba8();
    for p in out.pixels_mut() {
        let a = p[3] as u32;
        if let Some(alpha) = std::num::NonZeroU32::new(a) {
            for ch in 0..3 {
                p[ch] = ((p[ch] as u32 * 255 + a / 2) / alpha).min(255) as u8;
            }
        }
    }
    Ok(out)
}

fn rgb(p: &image::Rgba<u8>) -> [u8; 3] {
    [p[0], p[1], p[2]]
}

fn dist2(a: [u8; 3], b: [u8; 3]) -> u32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| {
            let d = *x as i32 - *y as i32;
            (d * d) as u32
        })
        .sum()
}

/// Index of the nearest colour among `candidates` (ties go to the lowest index).
fn nearest(
    colour: [u8; 3],
    palette: &[[u8; 3]],
    candidates: impl Iterator<Item = usize>,
) -> Option<usize> {
    candidates.min_by_key(|&i| (dist2(colour, palette[i]), i))
}

fn check(cancel: &dyn Fn() -> bool) -> Result<(), CoreError> {
    if cancel() {
        Err(CoreError::Cancelled)
    } else {
        Ok(())
    }
}

/// Deterministic colour reduction over the opaque pixels. Returns the palette
/// and one label per pixel.
fn reduce(
    img: &RgbaImage,
    n: usize,
    cancel: &dyn Fn() -> bool,
) -> Result<(Vec<[u8; 3]>, Vec<Label>), CoreError> {
    let opaque: Vec<[u8; 3]> = img.pixels().filter(|p| p[3] >= OPAQUE).map(rgb).collect();
    if opaque.is_empty() {
        return Ok((Vec::new(), vec![None; img.pixels().len()]));
    }
    let step = (opaque.len() / SAMPLE_TARGET).max(1);
    let sample: Vec<[u8; 3]> = opaque.iter().step_by(step).copied().collect();

    let mut distinct = Vec::new();
    let mut seen = HashSet::new();
    for c in &sample {
        if seen.insert(*c) {
            distinct.push(*c);
            if distinct.len() > n {
                break;
            }
        }
    }

    let palette = if distinct.len() <= n {
        distinct
    } else {
        kmeans(&sample, n, cancel)?
    };

    let width = img.width() as usize;
    let mut labels = Vec::with_capacity(img.pixels().len());
    for (i, p) in img.pixels().enumerate() {
        if i % width == 0 {
            check(cancel)?;
        }
        labels.push(if p[3] >= OPAQUE {
            nearest(rgb(p), &palette, 0..palette.len()).map(|c| c as u8)
        } else {
            None
        });
    }
    Ok((palette, labels))
}

/// Farthest-point seeding followed by Lloyd iterations.
fn kmeans(
    sample: &[[u8; 3]],
    n: usize,
    cancel: &dyn Fn() -> bool,
) -> Result<Vec<[u8; 3]>, CoreError> {
    let to_f = |c: [u8; 3]| c.map(|v| v as f64);
    let d2 = |a: [f64; 3], b: [f64; 3]| (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f64>();

    let mut centres = vec![to_f(sample[0])];
    let mut closest: Vec<f64> = sample.iter().map(|&c| d2(to_f(c), centres[0])).collect();
    while centres.len() < n {
        let mut best = 0;
        for (i, &d) in closest.iter().enumerate() {
            if d > closest[best] {
                best = i;
            }
        }
        let next = to_f(sample[best]);
        centres.push(next);
        for (i, &c) in sample.iter().enumerate() {
            closest[i] = closest[i].min(d2(to_f(c), next));
        }
    }

    for _ in 0..LLOYD_ITERATIONS {
        check(cancel)?;
        let mut sums = vec![[0.0f64; 3]; n];
        let mut counts = vec![0usize; n];
        for &c in sample {
            let p = to_f(c);
            let mut best = 0;
            let mut best_d = f64::INFINITY;
            for (k, centre) in centres.iter().enumerate() {
                let d = d2(p, *centre);
                if d < best_d {
                    best_d = d;
                    best = k;
                }
            }
            for ch in 0..3 {
                sums[best][ch] += p[ch];
            }
            counts[best] += 1;
        }
        for k in 0..n {
            if counts[k] > 0 {
                centres[k] = sums[k].map(|s| s / counts[k] as f64);
            }
        }
    }
    Ok(centres
        .iter()
        .map(|c| c.map(|v| v.round().clamp(0.0, 255.0) as u8))
        .collect())
}

/// Removes colours that are almost all edge (under 5% interior) from the label
/// map of the original small image, handing their pixels to their neighbours.
fn clean_thin_colours(labels: &mut [Label], w: usize, h: usize, palette: &[[u8; 3]]) {
    let mut removed = vec![false; palette.len()];
    let neighbours = |i: usize| {
        let (x, y) = (i % w, i / w);
        [
            (x > 0).then(|| i - 1),
            (x + 1 < w).then(|| i + 1),
            (y > 0).then(|| i - w),
            (y + 1 < h).then(|| i + w),
        ]
    };
    loop {
        let mut pixels = vec![0usize; palette.len()];
        let mut interior = vec![0usize; palette.len()];
        for (i, label) in labels.iter().enumerate() {
            if let Some(c) = *label {
                pixels[c as usize] += 1;
                let inner = neighbours(i)
                    .iter()
                    .flatten()
                    .all(|&j| labels[j] == Some(c));
                if inner {
                    interior[c as usize] += 1;
                }
            }
        }
        let live: Vec<usize> = (0..palette.len())
            .filter(|&c| !removed[c] && pixels[c] > 0)
            .collect();
        if live.len() <= 1 {
            return;
        }
        let thin = live
            .iter()
            .copied()
            .filter(|&c| interior[c] * 20 < pixels[c])
            .min_by(|&a, &b| {
                // interior/pixels compared by cross-multiplication; ties keep the lower index.
                (interior[a] * pixels[b])
                    .cmp(&(interior[b] * pixels[a]))
                    .then(a.cmp(&b))
            });
        let Some(victim) = thin else { return };
        removed[victim] = true;
        let survivors: Vec<usize> = live.iter().copied().filter(|&c| c != victim).collect();

        let mut orphans: Vec<usize> = (0..labels.len())
            .filter(|&i| labels[i] == Some(victim as u8))
            .collect();
        while !orphans.is_empty() {
            let snapshot = labels.to_vec();
            let mut still = Vec::new();
            for &i in &orphans {
                let mut votes = vec![0usize; palette.len()];
                for j in neighbours(i).into_iter().flatten() {
                    if let Some(c) = snapshot[j] {
                        if !removed[c as usize] {
                            votes[c as usize] += 1;
                        }
                    }
                }
                let best = (0..palette.len())
                    .filter(|&c| votes[c] > 0)
                    .max_by(|&a, &b| votes[a].cmp(&votes[b]).then(b.cmp(&a)));
                match best {
                    Some(c) => labels[i] = Some(c as u8),
                    None => still.push(i),
                }
            }
            if still.len() == orphans.len() {
                let fallback = nearest(palette[victim], palette, survivors.iter().copied())
                    .expect("more than one live colour");
                for &i in &still {
                    labels[i] = Some(fallback as u8);
                }
                break;
            }
            orphans = still;
        }
    }
}

/// Upscales the small image to `TRACE_MIN_SIDE`, letting each output pixel
/// pick only among the colours its source neighbourhood had.
fn guided_upscale(
    source: &RgbaImage,
    labels: &[Label],
    palette: &[[u8; 3]],
    cancel: &dyn Fn() -> bool,
) -> Result<(u32, u32, Vec<Label>), CoreError> {
    let target = resample(source, TRACE_MIN_SIDE)?;
    let (sw, sh) = (source.width() as usize, source.height() as usize);
    let (tw, th) = (target.width() as usize, target.height() as usize);
    let used: Vec<usize> = (0..palette.len())
        .filter(|&c| labels.contains(&Some(c as u8)))
        .collect();
    let mut out = Vec::with_capacity(tw * th);
    let coord = |t: usize, s: usize, tn: usize| {
        let f = ((t as f64 + 0.5) * s as f64 / tn as f64 - 0.5).max(0.0);
        let a = f.floor() as usize;
        (a.min(s - 1), (a + 1).min(s - 1))
    };
    for y in 0..th {
        check(cancel)?;
        let (y0, y1) = coord(y, sh, th);
        for x in 0..tw {
            let p = target.get_pixel(x as u32, y as u32);
            if p[3] < OPAQUE {
                out.push(None);
                continue;
            }
            let (x0, x1) = coord(x, sw, tw);
            let mut candidates: Vec<usize> = [(x0, y0), (x1, y0), (x0, y1), (x1, y1)]
                .iter()
                .filter_map(|&(cx, cy)| labels[cy * sw + cx].map(|c| c as usize))
                .collect();
            candidates.sort_unstable();
            candidates.dedup();
            let colour = rgb(p);
            let pick = if candidates.is_empty() {
                nearest(colour, palette, used.iter().copied())
            } else {
                nearest(colour, palette, candidates.into_iter())
            };
            out.push(pick.map(|c| c as u8));
        }
    }
    Ok((tw as u32, th as u32, out))
}

fn frame_indices(w: usize, h: usize) -> impl Iterator<Item = usize> {
    (0..w * h).filter(move |&i| {
        let (x, y) = (i % w, i / w);
        x == 0 || y == 0 || x + 1 == w || y + 1 == h
    })
}

fn hex(c: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

fn trace_color(
    source: &RgbaImage,
    n: usize,
    remove_background: bool,
    cancel: &dyn Fn() -> bool,
) -> Result<Traced, CoreError> {
    let long = longest_side(source);
    let (w, h, palette, mut labels) = if long < TRACE_MIN_SIDE {
        let (palette, mut labels) = reduce(source, n, cancel)?;
        clean_thin_colours(
            &mut labels,
            source.width() as usize,
            source.height() as usize,
            &palette,
        );
        let (w, h, labels) = guided_upscale(source, &labels, &palette, cancel)?;
        (w, h, palette, labels)
    } else {
        let resampled;
        let work = if long > TRACE_MAX_SIDE {
            resampled = resample(source, TRACE_MAX_SIDE)?;
            &resampled
        } else {
            source
        };
        let (palette, labels) = reduce(work, n, cancel)?;
        (work.width(), work.height(), palette, labels)
    };
    let (wu, hu) = (w as usize, h as usize);

    if remove_background {
        let mut counts = vec![0usize; palette.len()];
        let mut total = 0usize;
        for i in frame_indices(wu, hu) {
            total += 1;
            if let Some(c) = labels[i] {
                counts[c as usize] += 1;
            }
        }
        let dominant =
            (0..palette.len()).max_by(|&a, &b| counts[a].cmp(&counts[b]).then(b.cmp(&a)));
        if let Some(bg) = dominant {
            if counts[bg] * 2 >= total {
                for label in labels.iter_mut() {
                    if *label == Some(bg as u8) {
                        *label = None;
                    }
                }
            }
        }
    }

    let key = if labels.iter().any(Option::is_none) {
        let mut v: u32 = 1;
        loop {
            let c = [(v >> 16) as u8, (v >> 8) as u8, v as u8];
            if !palette.contains(&c) {
                break Color::new_rgba(c[0], c[1], c[2], 255);
            }
            v += 1;
        }
    } else {
        Color::default()
    };

    if labels.iter().all(Option::is_none) {
        return Ok(Traced {
            width: w,
            height: h,
            paths: Vec::new(),
        });
    }

    let mut image = ColorImage::new_w_h(wu, hu);
    for (i, label) in labels.iter().enumerate() {
        let c = match label {
            Some(c) => {
                let p = palette[*c as usize];
                Color::new_rgba(p[0], p[1], p[2], 255)
            }
            None => key,
        };
        image.set_pixel_at(i, &c);
    }

    let config = RunnerConfig {
        diagonal: false,
        hierarchical: HIERARCHICAL_MAX,
        batch_size: 25600,
        good_min_area: 16,
        good_max_area: wu * hu,
        is_same_color_a: 0,
        is_same_color_b: 1,
        deepen_diff: 16,
        hollow_neighbours: 1,
        key_color: key,
        keying_action: KeyingAction::Discard,
    };
    let paths = guard(|| {
        let mut builder = Runner::new(config, image).start();
        while !builder.tick() {
            check(cancel)?;
        }
        let clusters = builder.result();
        let view = clusters.view();
        let mut paths = Vec::new();
        for &index in view.clusters_output.iter().rev() {
            check(cancel)?;
            let cluster = view.get_cluster(index);
            let compound = cluster.to_compound_path(
                &view,
                false,
                PathSimplifyMode::Spline,
                CORNER_DEGREES.to_radians(),
                SEGMENT_LENGTH,
                MAX_ITERATIONS,
                SPLICE_DEGREES.to_radians(),
            );
            let d = path_data(&compound);
            if !d.is_empty() {
                let c = cluster.residue_color();
                paths.push((d, hex([c.r, c.g, c.b])));
            }
        }
        Ok(paths)
    })?;
    Ok(Traced {
        width: w,
        height: h,
        paths,
    })
}

fn trace_mono(source: &RgbaImage, cancel: &dyn Fn() -> bool) -> Result<Traced, CoreError> {
    let long = longest_side(source);
    let resampled;
    let work = if long > TRACE_MAX_SIDE {
        resampled = resample(source, TRACE_MAX_SIDE)?;
        &resampled
    } else if long < TRACE_MIN_SIDE {
        resampled = resample(source, TRACE_MIN_SIDE)?;
        &resampled
    } else {
        source
    };
    let (w, h) = (work.width() as usize, work.height() as usize);
    let opaque = |a: u8| a >= OPAQUE;
    // Composited on white: transparent pixels read as white, opaque ones
    // blend their RGB over white by their own alpha.
    let luma = |c: Color| -> u8 {
        if !opaque(c.a) {
            return 255;
        }
        let a = c.a as u32;
        let over_white = |v: u8| (v as u32 * a + 255 * (255 - a) + 127) / 255;
        ((299 * over_white(c.r) + 587 * over_white(c.g) + 114 * over_white(c.b)) / 1000) as u8
    };

    let frame: Vec<usize> = frame_indices(w, h).collect();
    let pixel = |i: usize| {
        let p = work.get_pixel((i % w) as u32, (i / w) as u32);
        Color::new_rgba(p[0], p[1], p[2], p[3])
    };
    let clear = frame.iter().filter(|&&i| !opaque(pixel(i).a)).count();

    // `None`: no foreground at all.
    let foreground: Option<Box<dyn Fn(Color) -> bool>> = if clear * 2 >= frame.len() {
        Some(Box::new(move |c: Color| opaque(c.a)))
    } else {
        let mut histogram = [0u64; 256];
        for i in 0..w * h {
            histogram[luma(pixel(i)) as usize] += 1;
        }
        let threshold = otsu(&histogram);
        threshold.map(|t| {
            let dark_in_frame = frame.iter().filter(|&&i| luma(pixel(i)) < t).count();
            let dark_is_background = dark_in_frame * 2 > frame.len();
            Box::new(move |c: Color| (luma(c) < t) != dark_is_background)
                as Box<dyn Fn(Color) -> bool>
        })
    };
    let Some(foreground) = foreground else {
        return Ok(Traced {
            width: w as u32,
            height: h as u32,
            paths: Vec::new(),
        });
    };

    let mut image = ColorImage::new_w_h(w, h);
    for i in 0..w * h {
        image.set_pixel_at(i, &pixel(i));
    }
    let d = guard(|| {
        let binary = image.to_binary_image(&foreground);
        let clusters = binary.to_clusters(false);
        let mut parts = Vec::new();
        for cluster in clusters.clusters.iter() {
            check(cancel)?;
            if cluster.points.len() < MONO_MIN_AREA {
                continue;
            }
            let compound = cluster.to_compound_path(
                PathSimplifyMode::Spline,
                CORNER_DEGREES.to_radians(),
                SEGMENT_LENGTH,
                MAX_ITERATIONS,
                SPLICE_DEGREES.to_radians(),
            );
            let d = path_data(&compound);
            if !d.is_empty() {
                parts.push(d);
            }
        }
        Ok(parts.join(" "))
    })?;
    let paths = if d.is_empty() {
        Vec::new()
    } else {
        vec![(d, "currentColor".to_string())]
    };
    Ok(Traced {
        width: w as u32,
        height: h as u32,
        paths,
    })
}

/// Otsu threshold `t` in `1..=255` (class 0 is `luma < t`; ties keep the lowest
/// `t`). `None` when the best split leaves a class empty.
fn otsu(histogram: &[u64; 256]) -> Option<u8> {
    let total: u64 = histogram.iter().sum();
    let weighted: f64 = histogram
        .iter()
        .enumerate()
        .map(|(v, &n)| v as f64 * n as f64)
        .sum();
    let (mut w0, mut sum0) = (0u64, 0f64);
    let mut best: Option<(f64, u8, u64)> = None;
    for t in 1..=255usize {
        w0 += histogram[t - 1];
        sum0 += (t - 1) as f64 * histogram[t - 1] as f64;
        let w1 = total - w0;
        let score = if w0 == 0 || w1 == 0 {
            0.0
        } else {
            let m0 = sum0 / w0 as f64;
            let m1 = (weighted - sum0) / w1 as f64;
            w0 as f64 * w1 as f64 * (m0 - m1).powi(2)
        };
        if best.is_none_or(|(s, _, _)| score > s) {
            best = Some((score, t as u8, w0));
        }
    }
    let (_, t, w0) = best?;
    (w0 > 0 && w0 < total).then_some(t)
}

/// Writes every element with absolute coordinates; `CompoundPath::to_svg_string`
/// would return an offset instead.
fn path_data(compound: &CompoundPath) -> String {
    let mut d = String::new();
    for element in compound.iter() {
        let part = match element {
            CompoundPathElement::Spline(s) => s.to_svg_string(true, &PointF64::default(), Some(1)),
            CompoundPathElement::PathF64(p) => p.to_svg_string(true, &PointF64::default(), Some(1)),
            CompoundPathElement::PathI32(p) => p.to_svg_string(true, &PointI32::default(), Some(1)),
        };
        d.push_str(&part);
    }
    d.trim_end().to_string()
}

/// `visioncortex` panics on some degenerate shapes (an invalid spline, for one).
fn guard<T>(work: impl FnOnce() -> Result<T, CoreError>) -> Result<T, CoreError> {
    catch_unwind(AssertUnwindSafe(work)).unwrap_or_else(|_| {
        Err(CoreError::Encode(
            "o vetorizador falhou nesta imagem".into(),
        ))
    })
}
