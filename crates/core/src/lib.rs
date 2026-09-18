//! Pure image engine. Knows nothing about Tauri, about Windows, or about any UI.

mod decode;
mod encode;
mod error;
mod resize;

pub use decode::{decode, detect_format, InputFormat};
pub use encode::{encode, EncodeSpec, OutputFormat};
pub use error::CoreError;
pub use resize::{resize, FitMode, ResizeSpec};
