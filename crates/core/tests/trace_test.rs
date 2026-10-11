use std::cell::Cell;

use image::{DynamicImage, RgbaImage};
use vdesigner_core::{rasterize_svg, trace_svg, CoreError, TraceMode, TraceSpec};

const RED: [u8; 4] = [220, 30, 30, 255];
const BLUE: [u8; 4] = [30, 30, 220, 255];
const NEVER: &dyn Fn() -> bool = &|| false;

fn color(n: u8, bg: bool) -> TraceSpec {
    TraceSpec {
        mode: TraceMode::Color,
        colors: n,
        remove_background: bg,
    }
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
    trace_svg(image, spec, (image.width(), image.height()), NEVER)
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
    let sized = text(&trace_svg(&white(1000, 500), &color(8, false), (64, 32), NEVER).unwrap());
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
    let result = trace_svg(&picture, &color(8, false), (640, 640), &cancel);
    assert!(matches!(result, Err(CoreError::Cancelled)));
    assert!(calls.get() > 3);
}
