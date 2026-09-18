mod common;

use vdesigner_core::{decode, detect_format, CoreError, InputFormat};

#[test]
fn detects_png_from_magic_bytes() {
    let bytes = common::as_png(&common::gradient(8, 8));
    assert_eq!(detect_format(&bytes).unwrap(), InputFormat::Png);
}

#[test]
fn decodes_png_preserving_dimensions() {
    let bytes = common::as_png(&common::gradient(32, 16));
    let img = decode(&bytes).unwrap();
    assert_eq!((img.width(), img.height()), (32, 16));
}

#[test]
fn rejects_unknown_bytes_with_unsupported_error() {
    let err = decode(b"not an image at all").unwrap_err();
    assert!(matches!(err, CoreError::UnsupportedFormat));
}

#[test]
fn rejects_truncated_png_with_decode_error() {
    let bytes = common::as_png(&common::gradient(8, 8));
    let truncated = &bytes[..bytes.len() / 2];
    assert!(matches!(decode(truncated), Err(CoreError::Decode(_))));
}
