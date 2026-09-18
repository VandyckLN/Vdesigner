//! Shared test helpers. Not every test binary that includes this module (via
//! `mod common;`, compiled per binary) uses every helper here.
#![allow(dead_code)]

use image::{DynamicImage, Rgba, RgbaImage};

/// Builds a deterministic gradient image, so tests never depend on binary fixtures.
pub fn gradient(width: u32, height: u32) -> DynamicImage {
    let mut img = RgbaImage::new(width, height);
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        let r = (x * 255 / width.max(1)) as u8;
        let g = (y * 255 / height.max(1)) as u8;
        *pixel = Rgba([r, g, 128, 255]);
    }
    DynamicImage::ImageRgba8(img)
}

/// Encodes an image with the `image` crate, used only to produce test input bytes.
pub fn as_png(img: &DynamicImage) -> Vec<u8> {
    let mut bytes = Vec::new();
    img.write_to(
        &mut std::io::Cursor::new(&mut bytes),
        image::ImageFormat::Png,
    )
    .expect("png encoding in test helper must not fail");
    bytes
}
