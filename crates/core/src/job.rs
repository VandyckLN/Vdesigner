use crate::{
    adjust, decode, denoise, encode, error::CoreError, is_svg, rasterize_svg, resize, sharpen,
    AdjustSpec, DenoiseSpec, EncodeSpec, FitMode, ResizeSpec, SharpenSpec,
};
use image::DynamicImage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Step {
    Resize(ResizeSpec),
    Sharpen(SharpenSpec),
    Denoise(DenoiseSpec),
    Adjust(AdjustSpec),
}

/// A complete operation, described as data. The Editor builds one; the batch
/// screen will build many from a preset. The engine cannot tell them apart.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub steps: Vec<Step>,
    pub output: EncodeSpec,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    /// 1-based index of the stage that just finished.
    pub current: u32,
    pub total: u32,
    pub label: &'static str,
}

#[derive(Debug, Clone)]
pub struct JobOutput {
    pub bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Runs the job at full resolution.
pub fn run_job(
    input: &[u8],
    job: &Job,
    progress: &mut dyn FnMut(Progress),
) -> Result<JobOutput, CoreError> {
    let image = load(input, job)?;
    execute(image, job, progress)
}

/// Runs the same job over a reduced copy, so the Editor stays interactive.
/// The reduction happens before the steps, and the final encode is unchanged,
/// which keeps a single code path between preview and export.
pub fn run_preview(input: &[u8], job: &Job, max_side: u32) -> Result<JobOutput, CoreError> {
    if max_side == 0 {
        return Err(CoreError::InvalidParameter(
            "max_side deve ser maior que zero".into(),
        ));
    }

    let image = load(input, job)?;
    let source_width = image.width().max(1);
    let longest = image.width().max(image.height());
    let reduced = if longest > max_side {
        let ratio = max_side as f64 / longest as f64;
        let spec = ResizeSpec {
            width: Some(((image.width() as f64 * ratio).round() as u32).max(1)),
            height: None,
            fit: FitMode::Contain,
            pad_color: [0, 0, 0, 0],
        };
        resize(&image, &spec)?
    } else {
        image
    };

    // A resize step inside the job would undo the reduction, so each one is
    // rescaled by the same factor the source was reduced by.
    let scale_ratio = reduced.width() as f64 / source_width as f64;
    let scaled_job = Job {
        steps: job.steps.iter().map(|s| scale_step(s, scale_ratio)).collect(),
        output: job.output,
    };

    execute(reduced, &scaled_job, &mut |_| {})
}

fn load(input: &[u8], job: &Job) -> Result<DynamicImage, CoreError> {
    if is_svg(input) {
        // Vector input is rasterized straight at the first resize target when
        // there is one, which avoids resampling a raster copy later.
        let target = job.steps.iter().find_map(|step| match step {
            Step::Resize(spec) => Some((spec.width, spec.height)),
            _ => None,
        });
        let (width, height) = target.unwrap_or((None, None));
        rasterize_svg(input, width, height)
    } else {
        decode(input)
    }
}

fn scale_step(step: &Step, ratio: f64) -> Step {
    match step {
        Step::Resize(spec) => Step::Resize(ResizeSpec {
            width: spec.width.map(|w| ((w as f64 * ratio).round() as u32).max(1)),
            height: spec.height.map(|h| ((h as f64 * ratio).round() as u32).max(1)),
            ..*spec
        }),
        other => other.clone(),
    }
}

fn execute(
    mut image: DynamicImage,
    job: &Job,
    progress: &mut dyn FnMut(Progress),
) -> Result<JobOutput, CoreError> {
    let total = job.steps.len() as u32 + 1;

    for (index, step) in job.steps.iter().enumerate() {
        let label = match step {
            Step::Resize(spec) => {
                image = resize(&image, spec)?;
                "redimensionando"
            }
            Step::Sharpen(spec) => {
                image = sharpen(&image, spec)?;
                "aplicando nitidez"
            }
            Step::Denoise(spec) => {
                image = denoise(&image, spec)?;
                "reduzindo ruído"
            }
            Step::Adjust(spec) => {
                image = adjust(&image, spec)?;
                "ajustando tom"
            }
        };
        progress(Progress { current: index as u32 + 1, total, label });
    }

    let bytes = encode(&image, &job.output)?;
    progress(Progress { current: total, total, label: "codificando" });

    Ok(JobOutput { bytes, width: image.width(), height: image.height() })
}
