use crate::error::CoreError;
use image::DynamicImage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    WebP,
    Avif,
    Png,
    Jpeg,
    Tiff,
    Ico,
}

#[derive(Debug, Clone, Copy)]
pub struct EncodeSpec {
    pub format: OutputFormat,
    /// 1 to 100. Ignored by the lossless formats.
    pub quality: u8,
    /// Only honoured by WebP, which supports both modes.
    pub lossless: bool,
}

/// The ICO container stores at most 256 pixels per side.
const ICO_MAX_SIDE: u32 = 256;

pub fn encode(img: &DynamicImage, spec: &EncodeSpec) -> Result<Vec<u8>, CoreError> {
    if spec.quality == 0 || spec.quality > 100 {
        return Err(CoreError::InvalidParameter(format!(
            "qualidade deve ficar entre 1 e 100, recebido {}",
            spec.quality
        )));
    }

    match spec.format {
        OutputFormat::WebP => encode_webp(img, spec),
        OutputFormat::Avif => encode_avif(img, spec),
        OutputFormat::Ico => {
            if img.width() > ICO_MAX_SIDE || img.height() > ICO_MAX_SIDE {
                return Err(CoreError::InvalidParameter(format!(
                    "ICO aceita no máximo {ICO_MAX_SIDE} pixels por lado, recebido {}x{}",
                    img.width(),
                    img.height()
                )));
            }
            encode_with_image_crate(img, image::ImageFormat::Ico)
        }
        OutputFormat::Png => encode_with_image_crate(img, image::ImageFormat::Png),
        OutputFormat::Tiff => encode_with_image_crate(img, image::ImageFormat::Tiff),
        OutputFormat::Jpeg => encode_jpeg(img, spec.quality),
    }
}

fn encode_with_image_crate(
    img: &DynamicImage,
    format: image::ImageFormat,
) -> Result<Vec<u8>, CoreError> {
    let mut bytes = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut bytes), format)
        .map_err(|e| CoreError::Encode(e.to_string()))?;
    Ok(bytes)
}

fn encode_jpeg(img: &DynamicImage, quality: u8) -> Result<Vec<u8>, CoreError> {
    // JPEG has no alpha channel, so the image is flattened over white first.
    let rgb = DynamicImage::ImageRgb8(img.to_rgb8());
    let mut bytes = Vec::new();
    let mut encoder =
        image::codecs::jpeg::JpegEncoder::new_with_quality(std::io::Cursor::new(&mut bytes), quality);
    encoder
        .encode_image(&rgb)
        .map_err(|e| CoreError::Encode(e.to_string()))?;
    Ok(bytes)
}

fn encode_webp(img: &DynamicImage, spec: &EncodeSpec) -> Result<Vec<u8>, CoreError> {
    let rgba = img.to_rgba8();
    let encoder = webp::Encoder::from_rgba(rgba.as_raw(), img.width(), img.height());
    let memory = if spec.lossless {
        encoder.encode_lossless()
    } else {
        encoder.encode(spec.quality as f32)
    };
    Ok(memory.to_vec())
}

fn encode_avif(img: &DynamicImage, spec: &EncodeSpec) -> Result<Vec<u8>, CoreError> {
    let rgba = img.to_rgba8();
    let pixels: Vec<rgb::RGBA8> = rgba
        .pixels()
        .map(|p| rgb::RGBA8::new(p.0[0], p.0[1], p.0[2], p.0[3]))
        .collect();
    let buffer = ravif::Img::new(pixels.as_slice(), img.width() as usize, img.height() as usize);
    let encoded = ravif::Encoder::new()
        .with_quality(spec.quality as f32)
        .with_speed(6)
        .encode_rgba(buffer)
        .map_err(|e| CoreError::Encode(e.to_string()))?;
    Ok(encoded.avif_file)
}
