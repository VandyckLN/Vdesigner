mod common;

use vdesigner_core::{resize, CoreError, FitMode, ResizeSpec};

fn spec(width: Option<u32>, height: Option<u32>, fit: FitMode) -> ResizeSpec {
    ResizeSpec { width, height, fit, pad_color: [0, 0, 0, 0] }
}

#[test]
fn width_only_keeps_the_aspect_ratio() {
    let img = common::gradient(400, 200);
    let out = resize(&img, &spec(Some(200), None, FitMode::Contain)).unwrap();
    assert_eq!((out.width(), out.height()), (200, 100));
}

#[test]
fn height_only_keeps_the_aspect_ratio() {
    let img = common::gradient(400, 200);
    let out = resize(&img, &spec(None, Some(50), FitMode::Contain)).unwrap();
    assert_eq!((out.width(), out.height()), (100, 50));
}

#[test]
fn contain_pads_to_the_exact_target_without_cropping() {
    let img = common::gradient(400, 200);
    let out = resize(&img, &spec(Some(100), Some(100), FitMode::Contain)).unwrap();
    assert_eq!((out.width(), out.height()), (100, 100));

    // The source is wider than tall, so the top row must be transparent padding.
    let rgba = out.to_rgba8();
    assert_eq!(rgba.get_pixel(50, 0).0[3], 0, "topo deve ser preenchimento");
    assert_ne!(rgba.get_pixel(50, 50).0[3], 0, "centro deve ser imagem");
}

#[test]
fn cover_fills_the_target_and_crops_the_overflow() {
    let img = common::gradient(400, 200);
    let out = resize(&img, &spec(Some(100), Some(100), FitMode::Cover)).unwrap();
    assert_eq!((out.width(), out.height()), (100, 100));

    let rgba = out.to_rgba8();
    for pixel in rgba.pixels() {
        assert_eq!(pixel.0[3], 255, "cover não deve deixar preenchimento");
    }
}

#[test]
fn stretch_distorts_on_purpose() {
    let img = common::gradient(400, 200);
    let out = resize(&img, &spec(Some(100), Some(100), FitMode::Stretch)).unwrap();
    assert_eq!((out.width(), out.height()), (100, 100));
}

#[test]
fn rejects_a_spec_without_any_dimension() {
    let img = common::gradient(10, 10);
    let err = resize(&img, &spec(None, None, FitMode::Contain)).unwrap_err();
    assert!(matches!(err, CoreError::InvalidParameter(_)));
}

#[test]
fn rejects_zero_as_a_dimension() {
    let img = common::gradient(10, 10);
    let err = resize(&img, &spec(Some(0), None, FitMode::Contain)).unwrap_err();
    assert!(matches!(err, CoreError::InvalidParameter(_)));
}

#[test]
fn upscaling_is_allowed() {
    let img = common::gradient(10, 10);
    let out = resize(&img, &spec(Some(40), None, FitMode::Contain)).unwrap();
    assert_eq!((out.width(), out.height()), (40, 40));
}
