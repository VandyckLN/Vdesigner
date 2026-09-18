use crate::error::CoreError;
use fast_image_resize::images::Image as FirImage;
use fast_image_resize::{FilterType, PixelType, ResizeAlg, ResizeOptions, Resizer};
use image::{DynamicImage, Rgba, RgbaImage};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FitMode {
    /// Fits inside the target and pads the remaining area. Never crops.
    Contain,
    /// Fills the target and crops the overflow, centred.
    Cover,
    /// Ignores the aspect ratio. Only reachable when the user unlocks it.
    Stretch,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct ResizeSpec {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fit: FitMode,
    /// RGBA padding colour, used by `Contain` only.
    pub pad_color: [u8; 4],
}

pub fn resize(img: &DynamicImage, spec: &ResizeSpec) -> Result<DynamicImage, CoreError> {
    validate(spec)?;

    let (src_w, src_h) = (img.width(), img.height());
    let (target_w, target_h) = target_size(src_w, src_h, spec);

    match spec.fit {
        FitMode::Stretch => scale(img, target_w, target_h),
        FitMode::Contain => {
            if spec.width.is_none() || spec.height.is_none() {
                // With a single dimension given, `target_size` already derived the other
                // one from the source ratio, so scaling straight to it is exact and no
                // padding is needed. Re-deriving the box via `fit_inside` here would use
                // a different rounding rule (round vs. `target_size`'s floor) and could
                // disagree by a pixel, producing a spurious pad on a lossless case.
                return scale(img, target_w, target_h);
            }
            let (inner_w, inner_h) = fit_inside(src_w, src_h, target_w, target_h);
            let scaled = scale(img, inner_w, inner_h)?;
            if (inner_w, inner_h) == (target_w, target_h) {
                return Ok(scaled);
            }
            Ok(pad_centered(&scaled, target_w, target_h, spec.pad_color))
        }
        FitMode::Cover => {
            let (outer_w, outer_h) = fill_outside(src_w, src_h, target_w, target_h);
            let scaled = scale(img, outer_w, outer_h)?;
            Ok(crop_centered(&scaled, target_w, target_h))
        }
    }
}

fn validate(spec: &ResizeSpec) -> Result<(), CoreError> {
    if spec.width.is_none() && spec.height.is_none() {
        return Err(CoreError::InvalidParameter(
            "informe ao menos largura ou altura".into(),
        ));
    }
    for value in [spec.width, spec.height].into_iter().flatten() {
        if value == 0 {
            return Err(CoreError::InvalidParameter(
                "as dimensões devem ser maiores que zero".into(),
            ));
        }
    }
    if spec.fit == FitMode::Stretch && (spec.width.is_none() || spec.height.is_none()) {
        return Err(CoreError::InvalidParameter(
            "o modo Stretch exige largura e altura".into(),
        ));
    }
    Ok(())
}

/// Resolves the requested box. A missing dimension is derived from the source ratio.
fn target_size(src_w: u32, src_h: u32, spec: &ResizeSpec) -> (u32, u32) {
    match (spec.width, spec.height) {
        (Some(w), Some(h)) => (w, h),
        (Some(w), None) => (w, scale_dimension(w, src_h, src_w)),
        (None, Some(h)) => (scale_dimension(h, src_w, src_h), h),
        (None, None) => (src_w, src_h),
    }
}

fn scale_dimension(known: u32, other_src: u32, known_src: u32) -> u32 {
    let value = (known as u64 * other_src as u64) / known_src.max(1) as u64;
    value.max(1) as u32
}

fn fit_inside(src_w: u32, src_h: u32, box_w: u32, box_h: u32) -> (u32, u32) {
    let ratio = f64::min(box_w as f64 / src_w as f64, box_h as f64 / src_h as f64);
    (
        ((src_w as f64 * ratio).round() as u32).max(1),
        ((src_h as f64 * ratio).round() as u32).max(1),
    )
}

fn fill_outside(src_w: u32, src_h: u32, box_w: u32, box_h: u32) -> (u32, u32) {
    let ratio = f64::max(box_w as f64 / src_w as f64, box_h as f64 / src_h as f64);
    (
        ((src_w as f64 * ratio).round() as u32).max(box_w),
        ((src_h as f64 * ratio).round() as u32).max(box_h),
    )
}

/// Lanczos3 rescale. This is the only place that touches the resampling library.
fn scale(img: &DynamicImage, width: u32, height: u32) -> Result<DynamicImage, CoreError> {
    let src = img.to_rgba8();
    let source = FirImage::from_vec_u8(img.width(), img.height(), src.into_raw(), PixelType::U8x4)
        .map_err(|e| CoreError::InvalidParameter(e.to_string()))?;
    let mut destination = FirImage::new(width, height, PixelType::U8x4);

    Resizer::new()
        .resize(
            &source,
            &mut destination,
            &ResizeOptions::new().resize_alg(ResizeAlg::Convolution(FilterType::Lanczos3)),
        )
        .map_err(|e| CoreError::InvalidParameter(e.to_string()))?;

    let buffer = RgbaImage::from_raw(width, height, destination.into_vec())
        .ok_or_else(|| CoreError::Encode("buffer redimensionado inválido".into()))?;
    Ok(DynamicImage::ImageRgba8(buffer))
}

fn pad_centered(img: &DynamicImage, width: u32, height: u32, color: [u8; 4]) -> DynamicImage {
    let mut canvas = RgbaImage::from_pixel(width, height, Rgba(color));
    let x = (width.saturating_sub(img.width())) / 2;
    let y = (height.saturating_sub(img.height())) / 2;
    image::imageops::overlay(&mut canvas, &img.to_rgba8(), x as i64, y as i64);
    DynamicImage::ImageRgba8(canvas)
}

fn crop_centered(img: &DynamicImage, width: u32, height: u32) -> DynamicImage {
    let x = (img.width().saturating_sub(width)) / 2;
    let y = (img.height().saturating_sub(height)) / 2;
    img.crop_imm(x, y, width, height)
}
