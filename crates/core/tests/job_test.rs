mod common;

use vdesigner_core::{
    run_job, run_preview, CoreError, DenoiseSpec, EncodeSpec, FitMode, Job, OutputFormat, Progress,
    ResizeSpec, SharpenSpec, Step,
};

fn png_input(width: u32, height: u32) -> Vec<u8> {
    common::as_png(&common::gradient(width, height))
}

fn webp_output() -> EncodeSpec {
    EncodeSpec { format: OutputFormat::WebP, quality: 82, lossless: false }
}

#[test]
fn runs_an_empty_job_as_a_pure_format_conversion() {
    let job = Job { steps: vec![], output: webp_output() };
    let out = run_job(&png_input(40, 20), &job, &mut |_| {}).unwrap();
    assert_eq!((out.width, out.height), (40, 20));
    assert!(!out.bytes.is_empty());
}

#[test]
fn applies_steps_in_order() {
    let job = Job {
        steps: vec![
            Step::Resize(ResizeSpec {
                width: Some(100),
                height: None,
                fit: FitMode::Contain,
                pad_color: [0, 0, 0, 0],
            }),
            Step::Sharpen(SharpenSpec { amount: 0.5, radius: 1.0 }),
        ],
        output: webp_output(),
    };
    let out = run_job(&png_input(400, 200), &job, &mut |_| {}).unwrap();
    assert_eq!((out.width, out.height), (100, 50));
}

#[test]
fn reports_progress_once_per_step_plus_encoding() {
    let job = Job {
        steps: vec![
            Step::Denoise(DenoiseSpec { radius: 1 }),
            Step::Sharpen(SharpenSpec { amount: 0.5, radius: 1.0 }),
        ],
        output: webp_output(),
    };
    let mut seen: Vec<Progress> = Vec::new();
    run_job(&png_input(32, 32), &job, &mut |p| seen.push(p)).unwrap();

    assert_eq!(seen.len(), 3, "duas etapas mais a codificação");
    assert_eq!(seen.last().unwrap().current, 3);
    assert!(seen.iter().all(|p| p.total == 3));
}

#[test]
fn accepts_svg_input_and_rasterizes_it() {
    let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10" width="10" height="10"><rect width="10" height="10" fill="#00ff00"/></svg>"##;
    let job = Job {
        steps: vec![Step::Resize(ResizeSpec {
            width: Some(64),
            height: None,
            fit: FitMode::Contain,
            pad_color: [0, 0, 0, 0],
        })],
        output: EncodeSpec { format: OutputFormat::Png, quality: 100, lossless: true },
    };
    let out = run_job(svg, &job, &mut |_| {}).unwrap();
    assert_eq!((out.width, out.height), (64, 64));
}

#[test]
fn preview_limits_the_longest_side() {
    let job = Job { steps: vec![], output: webp_output() };
    let out = run_preview(&png_input(4000, 2000), &job, 512).unwrap();
    assert_eq!(out.width, 512);
    assert_eq!(out.height, 256);
}

#[test]
fn preview_does_not_upscale_small_images() {
    let job = Job { steps: vec![], output: webp_output() };
    let out = run_preview(&png_input(100, 50), &job, 512).unwrap();
    assert_eq!((out.width, out.height), (100, 50));
}

#[test]
fn preview_keeps_an_explicit_resize_proportional_to_the_reduction() {
    // A preview of a job that resizes to 2000px must not return 2000px of pixels.
    let job = Job {
        steps: vec![Step::Resize(ResizeSpec {
            width: Some(2000),
            height: None,
            fit: FitMode::Contain,
            pad_color: [0, 0, 0, 0],
        })],
        output: webp_output(),
    };
    let out = run_preview(&png_input(4000, 2000), &job, 512).unwrap();
    assert!(out.width <= 512, "prévia não pode exceder o limite, veio {}", out.width);
}

#[test]
fn propagates_a_decode_failure() {
    let job = Job { steps: vec![], output: webp_output() };
    let err = run_job(b"garbage", &job, &mut |_| {}).unwrap_err();
    assert!(matches!(err, CoreError::UnsupportedFormat));
}

#[test]
fn job_round_trips_through_json() {
    let job = Job {
        steps: vec![Step::Denoise(DenoiseSpec { radius: 2 })],
        output: webp_output(),
    };
    let text = serde_json::to_string(&job).unwrap();
    let parsed: Job = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed.steps.len(), 1);
    assert_eq!(parsed.output.quality, 82);
}
