//! Pure image engine. Knows nothing about Tauri, about Windows, or about any UI.

mod decode;
mod encode;
mod error;
mod filters;
mod resize;
mod svg;

pub use decode::{decode, detect_format, InputFormat};
pub use encode::{encode, EncodeSpec, OutputFormat};
pub use error::CoreError;
pub use filters::{adjust, denoise, sharpen, AdjustSpec, DenoiseSpec, SharpenSpec};
pub use resize::{resize, FitMode, ResizeSpec};
pub use svg::{is_svg, rasterize_svg};
