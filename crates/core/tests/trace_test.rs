mod common;

use std::cell::Cell;

use image::{DynamicImage, RgbaImage};
use vdesigner_core::{rasterize_svg, trace_svg, CoreError, TraceDetail, TraceMode, TraceSpec};

const RED: [u8; 4] = [220, 30, 30, 255];
const BLUE: [u8; 4] = [30, 30, 220, 255];
const NEVER: &dyn Fn() -> bool = &|| false;
/// The job's `output.quality`. Only Embed reads it.
const QUALITY: u8 = 82;

fn color(n: u8, bg: bool) -> TraceSpec {
    TraceSpec {
        mode: TraceMode::Color,
        colors: n,
        remove_background: bg,
        ..TraceSpec::default()
    }
}

fn photo(detail: TraceDetail, bg: bool) -> TraceSpec {
    TraceSpec {
        mode: TraceMode::Photo,
        detail,
        colors: 99,
        remove_background: bg,
    }
}

/// Smooth two-axis gradient, a stand-in for a photo.
fn gradient(side: u32) -> DynamicImage {
    img(side, side, |x, y| {
        let u = x * 255 / (side - 1);
        let v = y * 255 / (side - 1);
        [u as u8, v as u8, ((u + v) / 2) as u8, 255]
    })
}

fn distinct(svg: &str) -> usize {
    fills(svg)
        .into_iter()
        .collect::<std::collections::HashSet<_>>()
        .len()
}

fn mae(a: &RgbaImage, b: &RgbaImage) -> f64 {
    let mut sum = 0u64;
    for (p, q) in a.pixels().zip(b.pixels()) {
        for ch in 0..3 {
            sum += p[ch].abs_diff(q[ch]) as u64;
        }
    }
    sum as f64 / (a.pixels().len() * 3) as f64
}

fn mono() -> TraceSpec {
    TraceSpec {
        mode: TraceMode::Mono,
        ..TraceSpec::default()
    }
}

fn img(w: u32, h: u32, f: impl Fn(u32, u32) -> [u8; 4]) -> DynamicImage {
    DynamicImage::ImageRgba8(RgbaImage::from_fn(w, h, |x, y| image::Rgba(f(x, y))))
}

fn trace(image: &DynamicImage, spec: &TraceSpec) -> Result<Vec<u8>, CoreError> {
    trace_svg(image, spec, QUALITY, (image.width(), image.height()), NEVER)
}

/// Sub-sample centres of a 4x4 grid inside pixel (x, y).
fn subsamples(x: u32, y: u32) -> impl Iterator<Item = (f64, f64)> {
    (0..16).map(move |i| {
        (
            x as f64 + ((i % 4) as f64 + 0.5) / 4.0,
            y as f64 + ((i / 4) as f64 + 0.5) / 4.0,
        )
    })
}

/// Antialiased disc over transparency: alpha is the 4x4 coverage, RGB is always the shape's.
fn soft_disc(side: u32, radius: f64, colour: [u8; 4]) -> DynamicImage {
    let c = side as f64 / 2.0;
    img(side, side, |x, y| {
        let covered = subsamples(x, y)
            .filter(|&(px, py)| (px - c).hypot(py - c) <= radius)
            .count() as u32;
        let a = ((covered * 255 + 8) / 16) as u8;
        [colour[0], colour[1], colour[2], a]
    })
}

fn logo(side: u32, noise: bool) -> DynamicImage {
    let s = side as f64;
    let shape = |px: f64, py: f64| -> [u8; 4] {
        let (u, v) = (px / s, py / s);
        let d = (u - 0.3).hypot(v - 0.5);
        let bar = (0.6..0.9).contains(&u)
            && [(0.30, 0.36), (0.45, 0.51), (0.60, 0.66)]
                .iter()
                .any(|&(a, b)| (a..b).contains(&v));
        if bar {
            [15, 15, 15, 255]
        } else if (0.22..0.27).contains(&d) {
            BLUE
        } else if d < 0.18 {
            RED
        } else {
            [255, 255, 255, 255]
        }
    };
    let mut state: u32 = 7;
    let mut rgba = RgbaImage::new(side, side);
    for (x, y, p) in rgba.enumerate_pixels_mut() {
        let mut sum = [0u32; 4];
        for (px, py) in subsamples(x, y) {
            for (acc, v) in sum.iter_mut().zip(shape(px, py)) {
                *acc += v as u32;
            }
        }
        *p = image::Rgba(sum.map(|v| ((v + 8) / 16) as u8));
        if noise {
            for ch in 0..3 {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let delta = ((state >> 16) % 9) as i32 - 4;
                p[ch] = (p[ch] as i32 + delta).clamp(0, 255) as u8;
            }
        }
    }
    DynamicImage::ImageRgba8(rgba)
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).unwrap()
}

fn back(bytes: &[u8], w: u32, h: u32) -> RgbaImage {
    rasterize_svg(bytes, Some(w), Some(h)).unwrap().to_rgba8()
}

fn paths(svg: &str) -> usize {
    svg.matches("<path").count()
}

fn fills(svg: &str) -> Vec<[u8; 3]> {
    svg.split("fill=\"#")
        .skip(1)
        .map(|rest| [0, 2, 4].map(|i| u8::from_str_radix(&rest[i..i + 2], 16).unwrap()))
        .collect()
}

fn near(a: [u8; 3], b: [u8; 3], tol: u8) -> bool {
    a.iter().zip(b.iter()).all(|(x, y)| x.abs_diff(*y) <= tol)
}

fn rgb(p: &image::Rgba<u8>) -> [u8; 3] {
    [p[0], p[1], p[2]]
}

fn rgb_of(c: [u8; 4]) -> [u8; 3] {
    [c[0], c[1], c[2]]
}

fn halves() -> DynamicImage {
    img(64, 64, |x, _| if x < 32 { RED } else { BLUE })
}

#[test]
fn traces_two_flat_colours_into_paths_that_rasterize_back_to_them() {
    let out = trace(&halves(), &color(8, false)).unwrap();
    let r = back(&out, 64, 64);
    assert!(near(rgb(r.get_pixel(16, 32)), rgb_of(RED), 8));
    assert!(near(rgb(r.get_pixel(48, 32)), rgb_of(BLUE), 8));
}

#[test]
fn keeps_flat_colours_exact_when_there_are_fewer_than_requested() {
    let out = text(&trace(&halves(), &color(8, false)).unwrap());
    let f = fills(&out);
    assert!(f.contains(&[220, 30, 30]), "{f:?}");
    assert!(f.contains(&[30, 30, 220]), "{f:?}");
}

#[test]
fn writes_the_output_size_and_a_viewbox_of_the_working_copy() {
    let white = |w, h| img(w, h, |_, _| [255, 255, 255, 255]);
    let small = text(&trace(&white(100, 50), &color(8, false)).unwrap());
    assert!(small.contains(r#"width="100" height="50" viewBox="0 0 512 256""#));
    let own = text(&trace(&white(1000, 500), &color(8, false)).unwrap());
    assert!(own.contains(r#"viewBox="0 0 1000 500""#));
    let large = text(&trace(&white(4100, 10), &color(8, false)).unwrap());
    assert!(large.contains(r#"viewBox="0 0 4096 10""#));
    let sized = text(
        &trace_svg(
            &white(1000, 500),
            &color(8, false),
            QUALITY,
            (64, 32),
            NEVER,
        )
        .unwrap(),
    );
    assert!(sized.contains(r#"width="64" height="32" viewBox="0 0 1000 500""#));
}

#[test]
fn writes_absolute_coordinates_without_transforms_or_metadata() {
    let out = text(&trace(&logo(256, true), &color(8, false)).unwrap());
    assert!(out.starts_with(r#"<svg xmlns="http://www.w3.org/2000/svg""#));
    assert!(!out.contains("transform"));
    assert!(!out.contains("<!--"));
    assert!(!out.contains("<metadata"));
}

#[test]
fn turns_transparent_pixels_into_empty_space() {
    let disc = img(64, 64, |x, y| {
        if (x as f64 - 32.0).hypot(y as f64 - 32.0) <= 20.0 {
            RED
        } else {
            [0, 0, 0, 0]
        }
    });
    let r = back(&trace(&disc, &color(8, false)).unwrap(), 64, 64);
    assert_eq!(r.get_pixel(2, 2)[3], 0);
    assert!(near(rgb(r.get_pixel(32, 32)), rgb_of(RED), 8));
}

#[test]
fn does_not_draw_a_dark_fringe_around_a_soft_edge_on_transparency() {
    let out = text(&trace(&soft_disc(64, 20.0, RED), &color(8, false)).unwrap());
    let f = fills(&out);
    assert!(!f.is_empty());
    for c in f {
        assert!(near(c, rgb_of(RED), 40), "{c:?}");
    }
}

#[test]
fn keeps_a_noisy_antialiased_logo_down_to_a_handful_of_paths() {
    for side in [256, 640] {
        let out = text(&trace(&logo(side, true), &color(8, false)).unwrap());
        assert!(paths(&out) <= 10, "{side}: {} paths", paths(&out));
    }
}

#[test]
fn traces_a_small_flat_logo_without_slivers() {
    let out = text(&trace(&logo(96, false), &color(8, false)).unwrap());
    assert!(paths(&out) <= 8, "{} paths", paths(&out));
    let allowed = [[255, 255, 255], rgb_of(RED), rgb_of(BLUE), [15, 15, 15]];
    for c in fills(&out) {
        assert!(allowed.iter().any(|a| near(c, *a, 24)), "{c:?}");
    }
}

#[test]
fn removes_the_border_colour_everywhere_when_asked() {
    let picture = img(64, 64, |x, y| {
        let inside = |a, b| (a..b).contains(&x) && (a..b).contains(&y);
        if inside(28, 36) {
            [255, 255, 255, 255]
        } else if inside(16, 48) {
            RED
        } else {
            [255, 255, 255, 255]
        }
    });
    let r = back(&trace(&picture, &color(8, true)).unwrap(), 64, 64);
    assert_eq!(r.get_pixel(2, 2)[3], 0);
    assert_eq!(r.get_pixel(32, 32)[3], 0);
    assert!(near(rgb(r.get_pixel(20, 20)), rgb_of(RED), 8));

    let r = back(&trace(&picture, &color(8, false)).unwrap(), 64, 64);
    assert_eq!(r.get_pixel(2, 2)[3], 255);
    assert!(near(rgb(r.get_pixel(2, 2)), [255, 255, 255], 8));
}

#[test]
fn keeps_the_background_when_no_colour_dominates_the_border() {
    let picture = img(64, 64, |x, y| match (x < 32, y < 32) {
        (true, true) => RED,
        (false, true) => BLUE,
        (true, false) => [30, 200, 30, 255],
        (false, false) => [240, 220, 20, 255],
    });
    let r = back(&trace(&picture, &color(8, true)).unwrap(), 64, 64);
    for (x, y) in [(0, 0), (63, 0), (0, 63), (63, 63)] {
        assert_eq!(r.get_pixel(x, y)[3], 255, "({x},{y})");
    }
}

#[test]
fn traces_mono_shapes_as_one_current_color_path() {
    let picture = img(64, 64, |x, y| {
        let d = (x as f64 - 32.0).hypot(y as f64 - 32.0);
        if d <= 4.0 || (12.0..=20.0).contains(&d) {
            [0, 0, 0, 255]
        } else {
            [255, 255, 255, 255]
        }
    });
    let out = trace(&picture, &mono()).unwrap();
    let s = text(&out);
    assert_eq!(paths(&s), 1);
    assert!(s.contains(r#"fill="currentColor""#));
    let r = back(&out, 64, 64);
    assert_eq!(r.get_pixel(32, 32)[3], 255);
    assert_eq!(r.get_pixel(32, 32 - 8)[3], 0);
    assert_eq!(r.get_pixel(32, 32 - 16)[3], 255);
}

#[test]
fn picks_light_strokes_on_a_dark_background_in_mono() {
    let picture = img(64, 64, |_, y| {
        if (28..36).contains(&y) {
            [255, 255, 255, 255]
        } else {
            [0, 0, 0, 255]
        }
    });
    let r = back(&trace(&picture, &mono()).unwrap(), 64, 64);
    assert_eq!(r.get_pixel(32, 32)[3], 255);
    assert_eq!(r.get_pixel(32, 4)[3], 0);
}

#[test]
fn uses_the_silhouette_of_a_transparent_image_in_mono() {
    let picture = img(64, 64, |x, y| {
        if (x as f64 - 32.0).hypot(y as f64 - 32.0) <= 20.0 {
            [255, 255, 255, 255]
        } else {
            [0, 0, 0, 0]
        }
    });
    let r = back(&trace(&picture, &mono()).unwrap(), 64, 64);
    assert_eq!(r.get_pixel(32, 32)[3], 255);
    assert_eq!(r.get_pixel(2, 2)[3], 0);
}

#[test]
fn produces_no_path_for_an_empty_picture() {
    let clear = trace(&img(32, 32, |_, _| [0, 0, 0, 0]), &color(8, false)).unwrap();
    assert_eq!(paths(&text(&clear)), 0);
    back(&clear, 32, 32);
    let white = trace(&img(32, 32, |_, _| [255, 255, 255, 255]), &mono()).unwrap();
    assert_eq!(paths(&text(&white)), 0);
    back(&white, 32, 32);
}

#[test]
fn traces_a_one_pixel_image() {
    let out = trace(&img(1, 1, |_, _| RED), &color(8, false)).unwrap();
    assert!(text(&out).contains(r#"viewBox="0 0 512 512""#));
    rasterize_svg(&out, None, None).unwrap();
}

#[test]
fn rejects_colour_counts_outside_two_to_sixteen() {
    let picture = halves();
    for n in [1, 17] {
        assert!(matches!(
            trace(&picture, &color(n, false)),
            Err(CoreError::InvalidParameter(_))
        ));
    }
    for n in [2, 16] {
        assert!(trace(&picture, &color(n, false)).is_ok());
    }
}

#[test]
fn is_deterministic() {
    for side in [256, 640] {
        let picture = logo(side, true);
        let a = trace(&picture, &color(8, false)).unwrap();
        let b = trace(&picture, &color(8, false)).unwrap();
        assert_eq!(a, b, "{side}");
    }
}

#[test]
fn stops_mid_trace_when_cancelled() {
    let calls = Cell::new(0u32);
    let cancel = || {
        calls.set(calls.get() + 1);
        calls.get() >= 4
    };
    let picture = logo(640, true);
    let result = trace_svg(&picture, &color(8, false), QUALITY, (640, 640), &cancel);
    assert!(matches!(result, Err(CoreError::Cancelled)));
    assert!(calls.get() > 3);
}

#[test]
fn photo_keeps_far_more_colours_and_reconstructs_closer_than_color() {
    let src = gradient(512);
    let poster = trace(&src, &color(16, false)).unwrap();
    let fiel = trace(&src, &photo(TraceDetail::Normal, false)).unwrap();
    assert!(distinct(&text(&fiel)) >= 3 * distinct(&text(&poster)));
    let rgba = src.to_rgba8();
    let e_poster = mae(&rgba, &back(&poster, 512, 512));
    let e_fiel = mae(&rgba, &back(&fiel, 512, 512));
    assert!(
        e_fiel < e_poster * 0.75,
        "foto {e_fiel} vs cores {e_poster}"
    );
}

#[test]
fn photo_never_removes_the_background() {
    let src = img(512, 512, |x, y| {
        let inside = (x as i32 - 256).abs() < 100 && (y as i32 - 256).abs() < 100;
        if inside {
            RED
        } else {
            [255, 255, 255, 255]
        }
    });
    let out = trace(&src, &photo(TraceDetail::Normal, true)).unwrap();
    let raster = back(&out, 512, 512);
    let corner = raster.get_pixel(3, 3);
    assert_eq!(corner[3], 255);
    assert!(near(rgb(corner), [255, 255, 255], 2));
}

#[test]
fn photo_detail_levels_grow_the_palette() {
    let src = gradient(512);
    let counts: Vec<usize> = [TraceDetail::Normal, TraceDetail::High, TraceDetail::Max]
        .into_iter()
        .map(|d| distinct(&text(&trace(&src, &photo(d, false)).unwrap())))
        .collect();
    assert!(
        counts[0] > 16 && counts[0] < counts[1] && counts[1] < counts[2],
        "{counts:?}"
    );
    assert!(counts[2] <= 255);
}

#[test]
fn photo_ignores_the_colour_count() {
    trace(&halves(), &photo(TraceDetail::Max, false)).unwrap();
}

#[test]
fn reads_old_json_without_detail() {
    let spec: TraceSpec =
        serde_json::from_str(r#"{"mode":"Color","colors":8,"remove_background":true}"#).unwrap();
    assert_eq!(spec.detail, TraceDetail::Normal);
    let spec: TraceSpec = serde_json::from_str(
        r#"{"mode":"Photo","detail":"High","colors":8,"remove_background":false}"#,
    )
    .unwrap();
    assert_eq!(
        (spec.mode, spec.detail),
        (TraceMode::Photo, TraceDetail::High)
    );
}

#[test]
fn photo_stops_when_cancelled() {
    let r = trace_svg(
        &gradient(512),
        &photo(TraceDetail::Normal, false),
        QUALITY,
        (512, 512),
        &|| true,
    );
    assert!(matches!(r, Err(CoreError::Cancelled)));
}

/// Every field Embed must ignore holds something that would fail or change a
/// traced result: a colour count out of range, Max detail, background removal.
fn embed() -> TraceSpec {
    TraceSpec {
        mode: TraceMode::Embed,
        colors: 99,
        remove_background: true,
        detail: TraceDetail::Max,
    }
}

fn embed_at(image: &DynamicImage, size: (u32, u32)) -> String {
    text(&trace_svg(image, &embed(), QUALITY, size, NEVER).unwrap())
}

/// The picture an Embed SVG carries, decoded.
fn inside(svg: &str) -> DynamicImage {
    image::load_from_memory(&common::embedded_picture(svg).1).unwrap()
}

/// Opaque, smooth and wider than tall: a photo stand-in that also catches a
/// swapped width and height.
fn landscape(w: u32, h: u32) -> DynamicImage {
    img(w, h, |x, y| {
        let u = x * 255 / (w - 1);
        let v = y * 255 / (h - 1);
        [u as u8, v as u8, ((u + v) / 2) as u8, 255]
    })
}

/// A disc over transparency with clear, soft and opaque pixels, and colour
/// left under the clear ones, so only an exact copy keeps every byte.
fn cutout(w: u32, h: u32) -> DynamicImage {
    let (cx, cy, r) = (w as f64 / 2.0, h as f64 / 2.0, h as f64 / 3.0);
    img(w, h, |x, y| {
        let covered = subsamples(x, y)
            .filter(|&(px, py)| (px - cx).hypot(py - cy) <= r)
            .count() as u32;
        [
            (x * 2) as u8,
            (y * 3) as u8,
            90,
            ((covered * 255 + 8) / 16) as u8,
        ]
    })
}

#[test]
fn embed_keeps_an_opaque_picture_as_jpeg() {
    let svg = embed_at(&landscape(300, 200), (300, 200));
    assert!(
        svg.starts_with(concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink""#,
            r#" width="300" height="200" viewBox="0 0 300 200">"#,
        )),
        "{}",
        &svg[..svg.len().min(300)]
    );
    assert!(svg.contains(r#"<image width="300" height="200" preserveAspectRatio="none" xlink:href="data:image/jpeg;base64,"#));
    assert!(svg.trim_end().ends_with("\"/>\n</svg>"));
    let (mime, bytes) = common::embedded_picture(&svg);
    assert_eq!(mime, "image/jpeg");
    assert_eq!(
        image::guess_format(&bytes).unwrap(),
        image::ImageFormat::Jpeg
    );
    let picture = inside(&svg);
    assert_eq!((picture.width(), picture.height()), (300, 200));
}

#[test]
fn embed_keeps_transparency_as_png_with_the_same_pixels() {
    let src = cutout(120, 80);
    let svg = embed_at(&src, (120, 80));
    assert!(svg.contains(r#"<image width="120" height="80" preserveAspectRatio="none" xlink:href="data:image/png;base64,"#));
    let (mime, bytes) = common::embedded_picture(&svg);
    assert_eq!(mime, "image/png");
    assert_eq!(
        image::guess_format(&bytes).unwrap(),
        image::ImageFormat::Png
    );
    let picture = inside(&svg).to_rgba8();
    assert_eq!(picture.dimensions(), (120, 80));
    assert!(
        picture.as_raw() == src.to_rgba8().as_raw(),
        "pixels mudaram"
    );
}

#[test]
fn embed_writes_the_requested_size_around_the_working_resolution() {
    let svg = embed_at(&landscape(1000, 500), (100, 50));
    assert!(svg.contains(r#" width="100" height="50" viewBox="0 0 100 50">"#));
    assert!(
        svg.contains(r#"<image width="100" height="50" preserveAspectRatio="none" xlink:href="#)
    );
    let picture = inside(&svg);
    assert_eq!((picture.width(), picture.height()), (1000, 500));
}

#[test]
fn embed_never_enlarges_a_small_picture() {
    let svg = embed_at(&landscape(100, 60), (100, 60));
    let picture = inside(&svg);
    assert_eq!((picture.width(), picture.height()), (100, 60));
}

#[test]
fn embed_reduces_past_the_trace_maximum_like_the_tracer() {
    let svg = embed_at(&landscape(4100, 10), (4100, 10));
    assert!(svg.contains(r#" width="4100" height="10" viewBox="0 0 4100 10">"#));
    let picture = inside(&svg);
    assert_eq!((picture.width(), picture.height()), (4096, 10));
}

#[test]
fn embed_covers_the_svg_edge_to_edge() {
    // Stored as 4096x10 inside a 4100x10 box. Kept to its own shape, the
    // picture would sit 2 px in from each end and leave both ends clear.
    let svg = embed_at(&landscape(4100, 10), (4100, 10));
    let r = back(svg.as_bytes(), 4100, 10);
    assert_eq!(r.get_pixel(0, 5)[3], 255);
    assert_eq!(r.get_pixel(4099, 5)[3], 255);
}

#[test]
fn embed_rasterizes_back_to_the_original() {
    let photo = landscape(300, 200);
    let e = mae(
        &photo.to_rgba8(),
        &back(embed_at(&photo, (300, 200)).as_bytes(), 300, 200),
    );
    assert!(e < 2.0, "jpeg {e}");

    // Hard edge over clear black: the rasterizer's premultiplied output then
    // reads the same as the straight source.
    let (cx, cy) = (60.0, 40.0);
    let disc = img(120, 80, |x, y| {
        if (x as f64 + 0.5 - cx).hypot(y as f64 + 0.5 - cy) <= 30.0 {
            [(x * 2) as u8, (y * 3) as u8, 90, 255]
        } else {
            [0, 0, 0, 0]
        }
    });
    let r = back(embed_at(&disc, (120, 80)).as_bytes(), 120, 80);
    let e = mae(&disc.to_rgba8(), &r);
    assert!(e < 2.0, "png {e}");
    assert_eq!(r.get_pixel(2, 2)[3], 0);
    assert_eq!(r.get_pixel(60, 40)[3], 255);
}

#[test]
fn embed_compresses_the_jpeg_with_the_job_quality() {
    let picture = logo(256, true);
    let size = |q| {
        trace_svg(&picture, &embed(), q, (256, 256), NEVER)
            .unwrap()
            .len()
    };
    assert!(size(40) < size(95), "{} vs {}", size(40), size(95));
}

#[test]
fn embed_ignores_the_colour_count() {
    for n in [0, 1, 17, 255] {
        let spec = TraceSpec {
            colors: n,
            ..embed()
        };
        assert!(
            trace_svg(&halves(), &spec, QUALITY, (64, 64), NEVER).is_ok(),
            "{n}"
        );
    }
}

#[test]
fn embed_checks_for_cancellation_before_and_after_encoding() {
    let picture = landscape(300, 200);
    let r = trace_svg(&picture, &embed(), QUALITY, (300, 200), &|| true);
    assert!(matches!(r, Err(CoreError::Cancelled)));

    // Lets the first check through, so the second one comes after encoding.
    let calls = Cell::new(0u32);
    let late = || {
        calls.set(calls.get() + 1);
        calls.get() >= 2
    };
    let r = trace_svg(&picture, &embed(), QUALITY, (300, 200), &late);
    assert!(matches!(r, Err(CoreError::Cancelled)));
    assert_eq!(calls.get(), 2);
}

#[test]
fn reads_embed_mode_from_json() {
    for json in [
        r#"{"mode":"Embed","colors":8,"remove_background":true,"detail":"Normal"}"#,
        r#"{"mode":"Embed","colors":8,"remove_background":true}"#,
    ] {
        let spec: TraceSpec = serde_json::from_str(json).unwrap();
        assert_eq!(spec.mode, TraceMode::Embed, "{json}");
    }
}
