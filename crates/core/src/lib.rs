//! Pure image engine. Knows nothing about Tauri, about Windows, or about any UI.

mod decode;
mod encode;
mod error;

pub use decode::{decode, detect_format, InputFormat};
pub use encode::{encode, EncodeSpec, OutputFormat};
pub use error::CoreError;
