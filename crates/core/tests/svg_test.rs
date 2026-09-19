use vdesigner_core::{is_svg, rasterize_svg, CoreError};

const CIRCLE: &[u8] = include_bytes!("fixtures/circle.svg");

#[test]
fn recognises_svg_bytes() {
    assert!(is_svg(CIRCLE));
    assert!(!is_svg(b"\x89PNG\r\n\x1a\n"));
}

#[test]
fn recognises_svg_with_leading_xml_declaration() {
    let with_prolog = b"<?xml version=\"1.0\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>";
    assert!(is_svg(with_prolog));
}

#[test]
fn rasterizes_at_the_intrinsic_size_when_no_dimension_is_given() {
    let img = rasterize_svg(CIRCLE, None, None).unwrap();
    assert_eq!((img.width(), img.height()), (100, 50));
}

#[test]
fn rasterizes_at_any_requested_size_keeping_the_ratio() {
    let img = rasterize_svg(CIRCLE, Some(400), None).unwrap();
    assert_eq!((img.width(), img.height()), (400, 200));
}

#[test]
fn renders_the_expected_colours() {
    let img = rasterize_svg(CIRCLE, None, None).unwrap();
    let rgba = img.to_rgba8();
    assert_eq!(
        rgba.get_pixel(50, 25).0[0..3],
        [255, 0, 0],
        "centro vermelho"
    );
    assert_eq!(
        rgba.get_pixel(2, 2).0[0..3],
        [255, 255, 255],
        "canto branco"
    );
}

#[test]
fn rejects_malformed_svg() {
    let err = rasterize_svg(b"<svg><unclosed>", None, None).unwrap_err();
    assert!(matches!(err, CoreError::Decode(_)));
}
