use crate::error::CoreError;
use image::DynamicImage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputFormat {
    Jpeg,
    Png,
    WebP,
    Gif,
    Bmp,
    Tiff,
}

/// Identifies the format from the leading bytes, never from the file extension.
pub fn detect_format(bytes: &[u8]) -> Result<InputFormat, CoreError> {
    match image::guess_format(bytes) {
        Ok(image::ImageFormat::Jpeg) => Ok(InputFormat::Jpeg),
        Ok(image::ImageFormat::Png) => Ok(InputFormat::Png),
        Ok(image::ImageFormat::WebP) => Ok(InputFormat::WebP),
        Ok(image::ImageFormat::Gif) => Ok(InputFormat::Gif),
        Ok(image::ImageFormat::Bmp) => Ok(InputFormat::Bmp),
        Ok(image::ImageFormat::Tiff) => Ok(InputFormat::Tiff),
        _ => Err(CoreError::UnsupportedFormat),
    }
}

/// Decodes raster input into RGBA8, the single working representation of the engine.
pub fn decode(bytes: &[u8]) -> Result<DynamicImage, CoreError> {
    let format = detect_format(bytes)?;
    let reader =
        image::ImageReader::with_format(std::io::Cursor::new(bytes), to_image_format(format));
    let decoded = reader
        .decode()
        .map_err(|e| CoreError::Decode(e.to_string()))?;
    Ok(DynamicImage::ImageRgba8(decoded.to_rgba8()))
}

fn to_image_format(format: InputFormat) -> image::ImageFormat {
    match format {
        InputFormat::Jpeg => image::ImageFormat::Jpeg,
        InputFormat::Png => image::ImageFormat::Png,
        InputFormat::WebP => image::ImageFormat::WebP,
        InputFormat::Gif => image::ImageFormat::Gif,
        InputFormat::Bmp => image::ImageFormat::Bmp,
        InputFormat::Tiff => image::ImageFormat::Tiff,
    }
}
