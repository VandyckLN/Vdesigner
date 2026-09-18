use crate::error::CoreError;
use image::{DynamicImage, Rgba, RgbaImage};

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct SharpenSpec {
    /// 0.0 disables the filter. Useful range goes up to 3.0.
    pub amount: f32,
    /// Blur radius of the unsharp mask, in pixels.
    pub radius: f32,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct DenoiseSpec {
    /// Median filter radius in pixels. 0 disables the filter.
    pub radius: u32,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct AdjustSpec {
    /// -1.0 to 1.0.
    pub brightness: f32,
    /// -1.0 to 1.0.
    pub contrast: f32,
    /// -1.0 fully desaturates, 1.0 doubles saturation.
    pub saturation: f32,
}

/// Unsharp mask: the image plus a weighted copy of its own high frequencies.
pub fn sharpen(img: &DynamicImage, spec: &SharpenSpec) -> Result<DynamicImage, CoreError> {
    if !(0.0..=5.0).contains(&spec.amount) {
        return Err(CoreError::InvalidParameter(
            "sharpen amount deve ficar entre 0.0 e 5.0".into(),
        ));
    }
    if spec.amount == 0.0 {
        return Ok(img.clone());
    }

    let blurred = img.blur(spec.radius.max(0.1));
    let original = img.to_rgba8();
    let blurred = blurred.to_rgba8();
    let mut out = RgbaImage::new(img.width(), img.height());

    for (x, y, pixel) in out.enumerate_pixels_mut() {
        let o = original.get_pixel(x, y).0;
        let b = blurred.get_pixel(x, y).0;
        let mut channels = [0u8; 4];
        for c in 0..3 {
            let detail = o[c] as f32 - b[c] as f32;
            channels[c] = clamp_u8(o[c] as f32 + detail * spec.amount);
        }
        channels[3] = o[3];
        *pixel = Rgba(channels);
    }

    Ok(DynamicImage::ImageRgba8(out))
}

/// Median filter. It removes speckle while keeping edges, which a blur would not.
pub fn denoise(img: &DynamicImage, spec: &DenoiseSpec) -> Result<DynamicImage, CoreError> {
    if spec.radius > 10 {
        return Err(CoreError::InvalidParameter(
            "denoise radius deve ficar entre 0 e 10".into(),
        ));
    }
    if spec.radius == 0 {
        return Ok(img.clone());
    }

    let filtered = imageproc::filter::median_filter(&img.to_rgba8(), spec.radius, spec.radius);
    Ok(DynamicImage::ImageRgba8(filtered))
}

pub fn adjust(img: &DynamicImage, spec: &AdjustSpec) -> Result<DynamicImage, CoreError> {
    for (name, value) in [
        ("brightness", spec.brightness),
        ("contrast", spec.contrast),
        ("saturation", spec.saturation),
    ] {
        if !(-1.0..=1.0).contains(&value) {
            return Err(CoreError::InvalidParameter(format!(
                "{name} deve ficar entre -1.0 e 1.0, recebido {value}"
            )));
        }
    }

    let source = img.to_rgba8();
    let mut out = RgbaImage::new(img.width(), img.height());
    let contrast_factor = 1.0 + spec.contrast;
    let brightness_offset = spec.brightness * 255.0;

    for (x, y, pixel) in out.enumerate_pixels_mut() {
        let p = source.get_pixel(x, y).0;
        let mut channels = [0f32; 3];
        for c in 0..3 {
            let value = p[c] as f32;
            // Contrast pivots around mid grey so the image does not drift dark.
            let with_contrast = (value - 127.5) * contrast_factor + 127.5;
            channels[c] = with_contrast + brightness_offset;
        }

        // Rec. 709 luminance, the same weighting browsers use for grayscale.
        let luma = 0.2126 * channels[0] + 0.7152 * channels[1] + 0.0722 * channels[2];
        let saturation_factor = 1.0 + spec.saturation;
        let final_channels = [
            clamp_u8(luma + (channels[0] - luma) * saturation_factor),
            clamp_u8(luma + (channels[1] - luma) * saturation_factor),
            clamp_u8(luma + (channels[2] - luma) * saturation_factor),
            p[3],
        ];
        *pixel = Rgba(final_channels);
    }

    Ok(DynamicImage::ImageRgba8(out))
}

fn clamp_u8(value: f32) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}
