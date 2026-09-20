//! Pure image engine. Knows nothing about Tauri, about Windows, or about any UI.

mod color;
mod decode;
mod encode;
mod error;
mod filters;
mod job;
mod palette;
mod resize;
mod svg;

pub use color::{
    format, from_oklch, gradient, harmonies, parse_hex, ramp, to_oklch, Color, ColorFormat,
    Harmony, Oklch, RAMP_STEPS,
};
pub use decode::{decode, detect_format, InputFormat};
pub use encode::{encode, EncodeSpec, OutputFormat};
pub use error::CoreError;
pub use filters::{adjust, denoise, sharpen, AdjustSpec, DenoiseSpec, SharpenSpec};
pub use job::{run_job, run_preview, Job, JobOutput, Progress, Step};
pub use palette::{
    palette_from_json, palette_to_json, to_css_vars, to_tailwind, validate_palette, Generated,
    GradientRef, Palette, Swatch, CSS_FILE_NAME, PALETTE_FILE_NAME, PALETTE_FORMAT_VERSION,
    TAILWIND_FILE_NAME,
};
pub use resize::{resize, FitMode, ResizeSpec, MAX_SIDE};
pub use svg::{is_svg, rasterize_svg};
