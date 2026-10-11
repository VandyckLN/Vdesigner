//! The picture itself inside an SVG, for when tracing would flatten it.

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use image::DynamicImage;

use crate::encode::{encode, EncodeSpec, OutputFormat};
use crate::error::CoreError;
use crate::trace::{check, resample, TraceSpec, TRACE_MAX_SIDE};

/// Wraps the picture in an SVG whose `width`, `height` and `viewBox` are
/// `size`. The pixels stay at the resolution the job worked at, reduced to
/// `TRACE_MAX_SIDE` like the tracer's copy. Unlike the tracer, nothing here
/// enlarges a small picture to `TRACE_MIN_SIDE`: an `<image>` gains nothing
/// from made-up pixels. The job's steps can still enlarge it, since
/// `detail_ratio` never runs them below the requested size: asking for more
/// than the source stores an enlarged picture, heavier and no sharper. A fully
/// opaque picture goes as JPEG at `quality`; any transparency goes as PNG,
/// which keeps every pixel.
///
/// `xlink:href` rather than `href`: Illustrator, Inkscape and Figma read the
/// former, and browsers read both.
///
/// `preserveAspectRatio="none"`: rounding at the working size, or in the
/// reduction, can leave the picture's shape a fraction of a pixel off
/// `size`, and the default `meet` would then leave clear columns or rows at
/// the edges. Stretching by that fraction does not show.
pub(crate) fn embed_svg(
    img: &DynamicImage,
    quality: u8,
    size: (u32, u32),
    cancel: &dyn Fn() -> bool,
) -> Result<Vec<u8>, CoreError> {
    check(cancel)?;
    let (format, mime) = if is_opaque(img) {
        (OutputFormat::Jpeg, "image/jpeg")
    } else {
        (OutputFormat::Png, "image/png")
    };
    let reduced;
    let work = if img.width().max(img.height()) > TRACE_MAX_SIDE {
        reduced = DynamicImage::ImageRgba8(resample(&img.to_rgba8(), TRACE_MAX_SIDE)?);
        &reduced
    } else {
        img
    };
    let picture = encode(
        work,
        &EncodeSpec {
            format,
            quality,
            lossless: false,
            trace: TraceSpec::default(),
        },
    )?;
    check(cancel)?;

    let (w, h) = size;
    let svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">\n"
    );
    let image = format!(
        "<image width=\"{w}\" height=\"{h}\" preserveAspectRatio=\"none\" xlink:href=\"data:{mime};base64,"
    );
    let end = "\"/>\n</svg>\n";
    // Base64 takes 4 characters for every 3 bytes, so this is the final
    // length, and the picture is encoded straight into the SVG, in one copy.
    let encoded_len = picture.len().div_ceil(3) * 4;
    let mut out = String::with_capacity(svg.len() + image.len() + encoded_len + end.len());
    out.push_str(&svg);
    out.push_str(&image);
    STANDARD.encode_string(&picture, &mut out);
    out.push_str(end);
    Ok(out.into_bytes())
}

/// JPEG has no alpha channel, so a single pixel short of opaque means PNG.
fn is_opaque(img: &DynamicImage) -> bool {
    if !img.color().has_alpha() {
        return true;
    }
    match img.as_rgba8() {
        Some(rgba) => rgba.pixels().all(|p| p[3] == u8::MAX),
        None => img.to_rgba8().pixels().all(|p| p[3] == u8::MAX),
    }
}
