mod common;

use image::{DynamicImage, Rgba, RgbaImage};
use vdesigner_core::{adjust, denoise, sharpen, AdjustSpec, CoreError, DenoiseSpec, SharpenSpec};

/// Mean absolute difference between neighbouring pixels along a row.
/// Sharpening raises it; denoising lowers it. This gives the tests a real
/// property to assert instead of comparing against a magic byte blob.
fn local_contrast(img: &DynamicImage) -> f64 {
    let rgba = img.to_rgba8();
    let mut total = 0f64;
    let mut count = 0u64;
    for y in 0..rgba.height() {
        for x in 1..rgba.width() {
            let a = rgba.get_pixel(x - 1, y).0[0] as f64;
            let b = rgba.get_pixel(x, y).0[0] as f64;
            total += (a - b).abs();
            count += 1;
        }
    }
    total / count.max(1) as f64
}

/// A checkerboard has strong local contrast, so filters show a clear effect.
fn checkerboard(size: u32) -> DynamicImage {
    let mut img = RgbaImage::new(size, size);
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        let value = if (x + y) % 2 == 0 { 20 } else { 235 };
        *pixel = Rgba([value, value, value, 255]);
    }
    DynamicImage::ImageRgba8(img)
}

#[test]
fn denoise_reduces_local_contrast() {
    let img = checkerboard(32);
    let before = local_contrast(&img);
    let after = local_contrast(&denoise(&img, &DenoiseSpec { radius: 1 }).unwrap());
    assert!(after < before, "denoise deveria suavizar: {after} < {before}");
}

#[test]
fn sharpen_increases_local_contrast_on_a_soft_image() {
    let soft = denoise(&checkerboard(32), &DenoiseSpec { radius: 2 }).unwrap();
    let before = local_contrast(&soft);
    let after = local_contrast(&sharpen(&soft, &SharpenSpec { amount: 1.5, radius: 1.0 }).unwrap());
    assert!(after > before, "sharpen deveria realçar: {after} > {before}");
}

#[test]
fn zero_amount_sharpen_returns_the_image_unchanged() {
    let img = common::gradient(16, 16);
    let out = sharpen(&img, &SharpenSpec { amount: 0.0, radius: 1.0 }).unwrap();
    assert_eq!(out.to_rgba8().into_raw(), img.to_rgba8().into_raw());
}

#[test]
fn zero_radius_denoise_returns_the_image_unchanged() {
    let img = common::gradient(16, 16);
    let out = denoise(&img, &DenoiseSpec { radius: 0 }).unwrap();
    assert_eq!(out.to_rgba8().into_raw(), img.to_rgba8().into_raw());
}

#[test]
fn brightness_raises_every_channel() {
    let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(4, 4, Rgba([100, 100, 100, 255])));
    let out = adjust(&img, &AdjustSpec { brightness: 0.2, contrast: 0.0, saturation: 0.0 }).unwrap();
    assert!(out.to_rgba8().get_pixel(0, 0).0[0] > 100);
}

#[test]
fn saturation_of_minus_one_produces_gray() {
    let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(4, 4, Rgba([200, 40, 40, 255])));
    let out = adjust(&img, &AdjustSpec { brightness: 0.0, contrast: 0.0, saturation: -1.0 }).unwrap();
    let pixel = out.to_rgba8().get_pixel(0, 0).0;
    assert!(
        pixel[0].abs_diff(pixel[1]) <= 2 && pixel[1].abs_diff(pixel[2]) <= 2,
        "esperava cinza, recebeu {pixel:?}"
    );
}

#[test]
fn adjust_preserves_alpha() {
    let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(4, 4, Rgba([10, 20, 30, 77])));
    let out = adjust(&img, &AdjustSpec { brightness: 0.5, contrast: 0.5, saturation: 0.5 }).unwrap();
    assert_eq!(out.to_rgba8().get_pixel(0, 0).0[3], 77);
}

#[test]
fn rejects_out_of_range_adjustments() {
    let img = common::gradient(4, 4);
    let err = adjust(&img, &AdjustSpec { brightness: 5.0, contrast: 0.0, saturation: 0.0 }).unwrap_err();
    assert!(matches!(err, CoreError::InvalidParameter(_)));
}
