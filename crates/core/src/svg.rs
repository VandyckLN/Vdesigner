use crate::error::CoreError;
use image::{DynamicImage, RgbaImage};
use resvg::tiny_skia;
use resvg::usvg;

/// Cheap sniff over the first bytes. SVG has no magic number, so the check looks
/// for the root tag, skipping an optional XML declaration and any whitespace.
pub fn is_svg(bytes: &[u8]) -> bool {
    let head = &bytes[..bytes.len().min(512)];
    String::from_utf8_lossy(head).contains("<svg")
}

/// Renders vector input into pixels at the requested size. A missing dimension is
/// derived from the intrinsic ratio, so vector art never distorts.
pub fn rasterize_svg(
    bytes: &[u8],
    width: Option<u32>,
    height: Option<u32>,
) -> Result<DynamicImage, CoreError> {
    let options = usvg::Options::default();
    let tree = usvg::Tree::from_data(bytes, &options)
        .map_err(|e| CoreError::Decode(format!("SVG inválido: {e}")))?;

    let intrinsic = tree.size();
    let (target_w, target_h) = match (width, height) {
        (Some(w), Some(h)) => (w, h),
        (Some(w), None) => {
            let ratio = intrinsic.height() / intrinsic.width();
            (w, ((w as f32 * ratio).round() as u32).max(1))
        }
        (None, Some(h)) => {
            let ratio = intrinsic.width() / intrinsic.height();
            (((h as f32 * ratio).round() as u32).max(1), h)
        }
        (None, None) => (
            intrinsic.width().round().max(1.0) as u32,
            intrinsic.height().round().max(1.0) as u32,
        ),
    };

    let mut pixmap = tiny_skia::Pixmap::new(target_w, target_h)
        .ok_or_else(|| CoreError::InvalidParameter("dimensões de SVG inválidas".into()))?;

    let transform = tiny_skia::Transform::from_scale(
        target_w as f32 / intrinsic.width(),
        target_h as f32 / intrinsic.height(),
    );
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    let buffer = RgbaImage::from_raw(target_w, target_h, pixmap.take())
        .ok_or_else(|| CoreError::Decode("buffer de SVG inválido".into()))?;
    Ok(DynamicImage::ImageRgba8(buffer))
}
