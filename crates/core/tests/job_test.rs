mod common;

use vdesigner_core::{
    run_job, run_job_cancellable, run_preview, CoreError, DenoiseSpec, EncodeSpec, FitMode, Job,
    OutputFormat, Progress, ResizeSpec, SharpenSpec, Step, TraceDetail, TraceMode, TraceSpec,
};

fn png_input(width: u32, height: u32) -> Vec<u8> {
    common::as_png(&common::gradient(width, height))
}

fn webp_output() -> EncodeSpec {
    EncodeSpec {
        format: OutputFormat::WebP,
        quality: 82,
        lossless: false,
        trace: TraceSpec::default(),
    }
}

#[test]
fn runs_an_empty_job_as_a_pure_format_conversion() {
    let job = Job {
        steps: vec![],
        output: webp_output(),
    };
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
            Step::Sharpen(SharpenSpec {
                amount: 0.5,
                radius: 1.0,
            }),
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
            Step::Sharpen(SharpenSpec {
                amount: 0.5,
                radius: 1.0,
            }),
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
        output: EncodeSpec {
            format: OutputFormat::Png,
            quality: 100,
            lossless: true,
            trace: TraceSpec::default(),
        },
    };
    let out = run_job(svg, &job, &mut |_| {}).unwrap();
    assert_eq!((out.width, out.height), (64, 64));
}

#[test]
fn preview_limits_the_longest_side() {
    let job = Job {
        steps: vec![],
        output: webp_output(),
    };
    let out = run_preview(&png_input(4000, 2000), &job, 512).unwrap();
    assert_eq!(out.width, 512);
    assert_eq!(out.height, 256);
}

#[test]
fn preview_does_not_upscale_small_images() {
    let job = Job {
        steps: vec![],
        output: webp_output(),
    };
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
    assert!(
        out.width <= 512,
        "prévia não pode exceder o limite, veio {}",
        out.width
    );
}

#[test]
fn propagates_a_decode_failure() {
    let job = Job {
        steps: vec![],
        output: webp_output(),
    };
    let err = run_job(b"garbage", &job, &mut |_| {}).unwrap_err();
    assert!(matches!(err, CoreError::UnsupportedFormat));
}

/// The literal the UI actually puts on the wire, copied byte for byte from
/// `WIRE_CONTRACT_JSON` in ui/src/api.test.ts, which asserts `buildJob`
/// serialises to exactly this. Renaming a field on either side now breaks a
/// test instead of silently desynchronising the two shapes.
#[test]
fn contract_json_from_the_ui_deserializes_into_a_job() {
    const WIRE_CONTRACT_JSON: &str = concat!(
        r#"{"steps":[{"Resize":{"width":800,"height":600,"fit":"Stretch","pad_color":[0,0,0,0]}},"#,
        r#"{"Denoise":{"radius":2}},{"Sharpen":{"amount":1,"radius":1}},"#,
        r#"{"Adjust":{"brightness":0.1,"contrast":0,"saturation":0}}],"#,
        r#""output":{"format":"WebP","quality":82,"lossless":false,"#,
        r#""trace":{"mode":"Color","colors":8,"remove_background":true,"detail":"High"}}}"#,
    );

    let job: Job = serde_json::from_str(WIRE_CONTRACT_JSON).expect("contrato de fio inválido");

    assert_eq!(
        job.output.trace,
        TraceSpec {
            mode: TraceMode::Color,
            colors: 8,
            remove_background: true,
            detail: TraceDetail::High,
        }
    );
    assert_eq!(job.steps.len(), 4);
    match &job.steps[0] {
        Step::Resize(spec) => {
            assert_eq!((spec.width, spec.height), (Some(800), Some(600)));
            assert_eq!(spec.fit, FitMode::Stretch);
            assert_eq!(spec.pad_color, [0, 0, 0, 0]);
        }
        other => panic!("esperava Resize, veio {other:?}"),
    }
    match &job.steps[1] {
        Step::Denoise(spec) => assert_eq!(spec.radius, 2),
        other => panic!("esperava Denoise, veio {other:?}"),
    }
    match &job.steps[2] {
        Step::Sharpen(spec) => {
            assert_eq!(spec.amount, 1.0);
            assert_eq!(spec.radius, 1.0);
        }
        other => panic!("esperava Sharpen, veio {other:?}"),
    }
    match &job.steps[3] {
        Step::Adjust(spec) => {
            assert_eq!(spec.brightness, 0.1);
            assert_eq!(spec.contrast, 0.0);
            assert_eq!(spec.saturation, 0.0);
        }
        other => panic!("esperava Adjust, veio {other:?}"),
    }
    assert_eq!(job.output.format, OutputFormat::WebP);
    assert_eq!(job.output.quality, 82);
    assert!(!job.output.lossless);
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

const CIRCLE: &[u8] = include_bytes!("fixtures/circle.svg");

fn svg_output() -> EncodeSpec {
    EncodeSpec {
        format: OutputFormat::Svg,
        quality: 82,
        lossless: false,
        trace: TraceSpec::default(),
    }
}

#[test]
fn refuses_to_vectorize_an_svg_input() {
    let job = Job {
        steps: vec![],
        output: svg_output(),
    };
    for result in [
        run_job(CIRCLE, &job, &mut |_| {}),
        run_preview(CIRCLE, &job, 512),
    ] {
        match result {
            Err(CoreError::InvalidParameter(m)) => assert!(m.contains("já é SVG"), "{m}"),
            other => panic!("esperava InvalidParameter, veio {other:?}"),
        }
    }
}

#[test]
fn labels_the_last_stage_vetorizando_for_svg() {
    let job = Job {
        steps: vec![],
        output: svg_output(),
    };
    let mut labels = Vec::new();
    run_job(&png_input(32, 32), &job, &mut |p| labels.push(p.label)).unwrap();
    assert_eq!(labels.last(), Some(&"vetorizando"));
}

#[test]
fn run_job_cancellable_stops_when_cancelled() {
    let job = Job {
        steps: vec![],
        output: svg_output(),
    };
    let result = run_job_cancellable(&png_input(64, 64), &job, &mut |_| {}, &|| true);
    assert!(matches!(result, Err(CoreError::Cancelled)));
}

#[test]
fn a_job_without_trace_still_deserializes() {
    let job: Job = serde_json::from_str(
        r#"{"steps":[],"output":{"format":"WebP","quality":82,"lossless":false}}"#,
    )
    .unwrap();
    assert_eq!(job.output.trace, TraceSpec::default());
}

#[test]
fn vectorizes_from_the_source_resolution_not_the_resize_target() {
    let job = Job {
        steps: vec![Step::Resize(ResizeSpec {
            width: Some(100),
            height: Some(50),
            fit: FitMode::Stretch,
            pad_color: [0, 0, 0, 0],
        })],
        output: svg_output(),
    };
    let out = run_job(&png_input(1000, 500), &job, &mut |_| {}).unwrap();
    let text = String::from_utf8(out.bytes).unwrap();
    assert!(
        text.contains(r#"width="100" height="50" viewBox="0 0 1000 500""#),
        "{}",
        &text[..text.len().min(300)]
    );
    assert_eq!((out.width, out.height), (100, 50));
}
