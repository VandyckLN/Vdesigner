use crate::{
    adjust, decode, denoise, encode,
    error::CoreError,
    is_svg, rasterize_svg, resize,
    resize::{fit_inside, target_size},
    sharpen, trace_svg, AdjustSpec, DenoiseSpec, EncodeSpec, FitMode, OutputFormat, ResizeSpec,
    SharpenSpec, TraceMode, TRACE_MAX_SIDE,
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
    run_job_cancellable(input, job, progress, &|| false)
}

/// Same as `run_job`, but `cancel` is polled between stages and inside the
/// tracer. For SVG output the steps run enlarged so the tracer, or the picture
/// Embed keeps, gets the source's detail, while the reported size stays the
/// one that was asked for.
pub fn run_job_cancellable(
    input: &[u8],
    job: &Job,
    progress: &mut dyn FnMut(Progress),
    cancel: &dyn Fn() -> bool,
) -> Result<JobOutput, CoreError> {
    let image = load(input, job)?;
    if job.output.format != OutputFormat::Svg {
        return execute(image, job, progress, cancel, None);
    }

    // Requested size: follow every Resize step on dimensions only.
    let (mut w, mut h) = (image.width(), image.height());
    let mut steps = Vec::with_capacity(job.steps.len());
    for step in &job.steps {
        let mut step = step.clone();
        if let Step::Resize(spec) = &mut step {
            let exact = fits_exactly(w, h, spec);
            (w, h) = target_size(w, h, spec);
            if exact {
                // Scaling both sides would round each apart, and the box could
                // come out a pixel or two larger than the fitted picture: a
                // clear pad the person never asked for, which turns Foto
                // original into a PNG. The long side alone derives the other.
                if spec.width >= spec.height {
                    spec.height = None;
                } else {
                    spec.width = None;
                }
            }
        }
        steps.push(step);
    }
    let ratio = detail_ratio(image.width().max(image.height()), w.max(h));
    let scaled_job = Job {
        steps: steps.iter().map(|s| scale_step(s, ratio)).collect(),
        output: job.output,
    };
    execute(image, &scaled_job, progress, cancel, Some((w, h)))
}

/// A Contain box with both sides that the picture fills at the requested size,
/// so it pads nothing.
fn fits_exactly(w: u32, h: u32, spec: &ResizeSpec) -> bool {
    match (spec.fit, spec.width, spec.height) {
        (FitMode::Contain, Some(bw), Some(bh)) => fit_inside(w, h, bw, bh) == (bw, bh),
        _ => false,
    }
}

/// How much larger than the requested size an SVG job's resize steps run, so
/// the tracer sees the source's detail instead of a thumbnail. Never below 1:
/// an enlargement the person asked for is kept. Never past TRACE_MAX_SIDE.
fn detail_ratio(source_long: u32, output_long: u32) -> f64 {
    let output = output_long.max(1) as f64;
    let cap = (TRACE_MAX_SIDE as f64 / output).max(1.0);
    (source_long as f64 / output).clamp(1.0, cap)
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
        steps: job
            .steps
            .iter()
            .map(|s| scale_step(s, scale_ratio))
            .collect(),
        output: job.output,
    };

    execute(reduced, &scaled_job, &mut |_| {}, &|| false, None)
}

fn load(input: &[u8], job: &Job) -> Result<DynamicImage, CoreError> {
    if is_svg(input) {
        if job.output.format == OutputFormat::Svg {
            return Err(CoreError::InvalidParameter(
                "a imagem já é SVG; escolha outro formato de saída".into(),
            ));
        }
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

/// Rescales every pixel-space quantity in a step by the factor the source was
/// reduced by, so the preview shows the same *effective* operation the export
/// will perform. Filter radii count: they are documented in pixels, so leaving
/// them alone would make a 2px blur far stronger relative to a reduced image
/// than to the full-resolution one.
fn scale_step(step: &Step, ratio: f64) -> Step {
    match step {
        Step::Resize(spec) => Step::Resize(ResizeSpec {
            width: spec
                .width
                .map(|w| ((w as f64 * ratio).round() as u32).max(1)),
            height: spec
                .height
                .map(|h| ((h as f64 * ratio).round() as u32).max(1)),
            ..*spec
        }),
        Step::Sharpen(spec) => Step::Sharpen(SharpenSpec {
            // No floor needed: `sharpen` already clamps the blur radius at 0.1.
            radius: spec.radius * ratio as f32,
            ..*spec
        }),
        Step::Denoise(spec) => Step::Denoise(DenoiseSpec {
            // An integer radius that rounds down to zero would silently disable
            // the filter in the preview while the export still applies it, so a
            // radius that was active stays active.
            radius: if spec.radius == 0 {
                0
            } else {
                ((spec.radius as f64 * ratio).round() as u32).max(1)
            },
        }),
        other => other.clone(),
    }
}

fn execute(
    mut image: DynamicImage,
    job: &Job,
    progress: &mut dyn FnMut(Progress),
    cancel: &dyn Fn() -> bool,
    size: Option<(u32, u32)>,
) -> Result<JobOutput, CoreError> {
    let total = job.steps.len() as u32 + 1;

    for (index, step) in job.steps.iter().enumerate() {
        if cancel() {
            return Err(CoreError::Cancelled);
        }
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
        progress(Progress {
            current: index as u32 + 1,
            total,
            label,
        });
    }

    let svg = job.output.format == OutputFormat::Svg;
    let bytes = if svg {
        let size = size.unwrap_or((image.width(), image.height()));
        trace_svg(&image, &job.output.trace, job.output.quality, size, cancel)?
    } else {
        encode(&image, &job.output)?
    };
    // Embed only encodes the picture it keeps, so it says so.
    let traced = svg && job.output.trace.mode != TraceMode::Embed;
    progress(Progress {
        current: total,
        total,
        label: if traced { "vetorizando" } else { "codificando" },
    });

    Ok(JobOutput {
        bytes,
        width: size.map_or(image.width(), |s| s.0),
        height: size.map_or(image.height(), |s| s.1),
    })
}

/// `scale_step` is private and its whole job is to return values, not pixels,
/// so it is checked here rather than through `run_preview`'s encoded output.
#[cfg(test)]
mod tests {
    use super::*;

    fn resize_step(width: Option<u32>, height: Option<u32>) -> Step {
        Step::Resize(ResizeSpec {
            width,
            height,
            fit: FitMode::Contain,
            pad_color: [0, 0, 0, 0],
        })
    }

    #[test]
    fn scales_resize_dimensions() {
        let scaled = scale_step(&resize_step(Some(2000), Some(1000)), 0.25);
        match scaled {
            Step::Resize(spec) => assert_eq!((spec.width, spec.height), (Some(500), Some(250))),
            other => panic!("esperava Resize, veio {other:?}"),
        }
    }

    #[test]
    fn keeps_a_resize_dimension_at_one_pixel_minimum() {
        let scaled = scale_step(&resize_step(Some(3), None), 0.01);
        match scaled {
            Step::Resize(spec) => assert_eq!((spec.width, spec.height), (Some(1), None)),
            other => panic!("esperava Resize, veio {other:?}"),
        }
    }

    #[test]
    fn scales_the_sharpen_radius_and_leaves_the_amount_alone() {
        let scaled = scale_step(
            &Step::Sharpen(SharpenSpec {
                amount: 0.8,
                radius: 4.0,
            }),
            0.25,
        );
        match scaled {
            Step::Sharpen(spec) => {
                assert!(
                    (spec.radius - 1.0).abs() < f32::EPSILON,
                    "raio deveria virar 1.0, veio {}",
                    spec.radius
                );
                assert!((spec.amount - 0.8).abs() < f32::EPSILON);
            }
            other => panic!("esperava Sharpen, veio {other:?}"),
        }
    }

    #[test]
    fn scales_the_denoise_radius() {
        let scaled = scale_step(&Step::Denoise(DenoiseSpec { radius: 8 }), 0.5);
        match scaled {
            Step::Denoise(spec) => assert_eq!(spec.radius, 4),
            other => panic!("esperava Denoise, veio {other:?}"),
        }
    }

    #[test]
    fn never_scales_an_active_denoise_radius_down_to_zero() {
        // Rounding to 0 would disable the filter in the preview while the
        // export still applied it, which is the worse divergence of the two.
        let scaled = scale_step(&Step::Denoise(DenoiseSpec { radius: 2 }), 0.05);
        match scaled {
            Step::Denoise(spec) => assert_eq!(spec.radius, 1),
            other => panic!("esperava Denoise, veio {other:?}"),
        }
    }

    #[test]
    fn keeps_a_disabled_denoise_disabled() {
        let scaled = scale_step(&Step::Denoise(DenoiseSpec { radius: 0 }), 0.5);
        match scaled {
            Step::Denoise(spec) => assert_eq!(spec.radius, 0),
            other => panic!("esperava Denoise, veio {other:?}"),
        }
    }

    #[test]
    fn leaves_adjust_untouched_because_it_has_no_pixel_space_values() {
        let scaled = scale_step(
            &Step::Adjust(AdjustSpec {
                brightness: 0.2,
                contrast: -0.1,
                saturation: 0.5,
            }),
            0.25,
        );
        match scaled {
            Step::Adjust(spec) => {
                assert!((spec.brightness - 0.2).abs() < f32::EPSILON);
                assert!((spec.contrast + 0.1).abs() < f32::EPSILON);
                assert!((spec.saturation - 0.5).abs() < f32::EPSILON);
            }
            other => panic!("esperava Adjust, veio {other:?}"),
        }
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn detail_ratio_brings_a_reduced_output_back_to_the_source_resolution() {
        assert!(close(detail_ratio(1000, 100), 10.0));
    }

    #[test]
    fn detail_ratio_stops_at_the_trace_cap() {
        assert!(close(detail_ratio(8000, 100), 40.96));
    }

    #[test]
    fn detail_ratio_never_shrinks_what_the_person_asked_for() {
        assert!(close(detail_ratio(500, 1000), 1.0));
        assert!(close(detail_ratio(100, 5000), 1.0));
    }
}
