mod common;

use vdesigner_core::{decode, detect_format, encode, CoreError, EncodeSpec, InputFormat, OutputFormat};

fn spec(format: OutputFormat, quality: u8) -> EncodeSpec {
    EncodeSpec { format, quality, lossless: false }
}

#[test]
fn encodes_webp_that_decodes_back_to_the_same_size() {
    let img = common::gradient(64, 48);
    let bytes = encode(&img, &spec(OutputFormat::WebP, 82)).unwrap();
    assert_eq!(detect_format(&bytes).unwrap(), InputFormat::WebP);
    let round_trip = decode(&bytes).unwrap();
    assert_eq!((round_trip.width(), round_trip.height()), (64, 48));
}

#[test]
fn encodes_png_losslessly() {
    let img = common::gradient(16, 16);
    let bytes = encode(&img, &spec(OutputFormat::Png, 100)).unwrap();
    let round_trip = decode(&bytes).unwrap();
    assert_eq!(round_trip.to_rgba8().into_raw(), img.to_rgba8().into_raw());
}

#[test]
fn lower_quality_produces_smaller_jpeg() {
    let img = common::gradient(256, 256);
    let high = encode(&img, &spec(OutputFormat::Jpeg, 95)).unwrap();
    let low = encode(&img, &spec(OutputFormat::Jpeg, 40)).unwrap();
    assert!(low.len() < high.len(), "qualidade menor deve gerar arquivo menor");
}

#[test]
fn encodes_avif() {
    let img = common::gradient(32, 32);
    let bytes = encode(&img, &spec(OutputFormat::Avif, 60)).unwrap();
    assert!(!bytes.is_empty());
}

#[test]
fn encodes_ico_and_tiff() {
    let img = common::gradient(32, 32);
    assert!(!encode(&img, &spec(OutputFormat::Ico, 100)).unwrap().is_empty());
    assert!(!encode(&img, &spec(OutputFormat::Tiff, 100)).unwrap().is_empty());
}

#[test]
fn rejects_ico_larger_than_256_pixels() {
    let img = common::gradient(512, 512);
    let err = encode(&img, &spec(OutputFormat::Ico, 100)).unwrap_err();
    assert!(matches!(err, CoreError::InvalidParameter(_)));
}

#[test]
fn rejects_quality_above_one_hundred() {
    let img = common::gradient(8, 8);
    let err = encode(&img, &spec(OutputFormat::Jpeg, 120)).unwrap_err();
    assert!(matches!(err, CoreError::InvalidParameter(_)));
}
