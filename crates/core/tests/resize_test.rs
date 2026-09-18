mod common;

use vdesigner_core::{resize, CoreError, FitMode, ResizeSpec};

fn spec(width: Option<u32>, height: Option<u32>, fit: FitMode) -> ResizeSpec {
    spec_with_pad(width, height, fit, [0, 0, 0, 0])
}

fn spec_with_pad(
    width: Option<u32>,
    height: Option<u32>,
    fit: FitMode,
    pad_color: [u8; 4],
) -> ResizeSpec {
    ResizeSpec { width, height, fit, pad_color }
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
fn single_dimension_contain_needs_no_padding_even_with_an_uneven_ratio() {
    // 7x8 -> width=5 derives height=floor(5*8/7)=5 via target_size's integer
    // division, while fit_inside's rounded arithmetic would compute inner=(4, 5)
    // for that same 5x5 box and wrongly pad a 1px column. Only one dimension was
    // requested, so the result must be an exact 5x5 scale with no padding at all.
    let img = common::gradient(7, 8);
    let out = resize(&img, &spec(Some(5), None, FitMode::Contain)).unwrap();
    assert_eq!((out.width(), out.height()), (5, 5));

    let rgba = out.to_rgba8();
    for pixel in rgba.pixels() {
        assert_eq!(
            pixel.0[3], 255,
            "redimensionamento de uma única dimensão não deve gerar preenchimento"
        );
    }
}

#[test]
fn contain_pads_using_the_requested_color() {
    let img = common::gradient(400, 200);
    let red = [255, 0, 0, 255];
    let out = resize(
        &img,
        &spec_with_pad(Some(100), Some(100), FitMode::Contain, red),
    )
    .unwrap();
    assert_eq!((out.width(), out.height()), (100, 100));

    // Same geometry as `contain_pads_to_the_exact_target_without_cropping`: the
    // source is wider than tall, so the top row is padding. It must carry the
    // caller's exact colour, not just be transparent.
    let rgba = out.to_rgba8();
    assert_eq!(
        rgba.get_pixel(50, 0).0,
        red,
        "preenchimento deve usar a cor solicitada"
    );
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
