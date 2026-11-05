use crate::error::CoreError;
use crate::resize::MAX_SIDE;
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
    let mut options = usvg::Options::default();
    // Forbid loading local files or URLs referenced by `<image href="...">`. The
    // default resolver reads them off disk, so a crafted SVG could pull an
    // arbitrary file into a preview or export. `data:` URIs still work through
    // the default data resolver.
    options.image_href_resolver.resolve_string = Box::new(|_href, _opts| None);
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

    // A hostile SVG can declare an enormous intrinsic size; without a cap the
    // pixmap allocation below reaches many gigabytes and aborts the process on
    // OOM. The same per-side limit the resizer enforces applies here.
    if target_w > MAX_SIDE || target_h > MAX_SIDE {
        return Err(CoreError::InvalidParameter(format!(
            "SVG grande demais: {target_w}x{target_h} excede {MAX_SIDE} pixels por lado"
        )));
    }

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
