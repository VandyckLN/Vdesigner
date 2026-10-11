use std::fs;
use vdesigner::commands::extrapolate;
use vdesigner::export::{resolve_output_path, ExportError};

#[test]
fn builds_the_path_from_the_stem_and_the_format_extension() {
    let dir = tempfile::tempdir().unwrap();
    let path = resolve_output_path(dir.path(), "hero", "webp", false).unwrap();
    assert_eq!(path.file_name().unwrap(), "hero.webp");
}

#[test]
fn refuses_to_overwrite_unless_explicitly_allowed() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("hero.webp"), b"existing").unwrap();

    let err = resolve_output_path(dir.path(), "hero", "webp", false).unwrap_err();
    assert!(matches!(err, ExportError::WouldOverwrite(_)));
}

#[test]
fn overwrites_when_explicitly_allowed() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("hero.webp"), b"existing").unwrap();

    let path = resolve_output_path(dir.path(), "hero", "webp", true).unwrap();
    assert_eq!(path.file_name().unwrap(), "hero.webp");
}

#[test]
fn rejects_a_stem_that_escapes_the_output_directory() {
    let dir = tempfile::tempdir().unwrap();
    let err = resolve_output_path(dir.path(), "../outside", "webp", true).unwrap_err();
    assert!(matches!(err, ExportError::InvalidName(_)));
}

#[test]
fn rejects_an_empty_stem() {
    let dir = tempfile::tempdir().unwrap();
    let err = resolve_output_path(dir.path(), "   ", "webp", true).unwrap_err();
    assert!(matches!(err, ExportError::InvalidName(_)));
}

#[test]
fn rejects_a_directory_that_does_not_exist() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("nao-existe");

    let err = resolve_output_path(&missing, "hero", "webp", false).unwrap_err();
    assert!(matches!(err, ExportError::InvalidDirectory(_)));
}

#[test]
fn scales_the_sample_by_the_square_of_the_reduction() {
    // 2048 is four times 512, so the full image holds sixteen times the pixels.
    assert_eq!(extrapolate(1_000, 2048, 1024, 512), 16_000);
}

#[test]
fn leaves_a_sample_alone_when_the_source_was_never_reduced() {
    assert_eq!(extrapolate(1_000, 400, 300, 512), 1_000);
}

#[test]
fn measures_the_reduction_against_the_longest_side() {
    // The portrait's height drives the cap, not its width.
    assert_eq!(extrapolate(1_000, 512, 1024, 512), 4_000);
}

fn two_color_png() -> Vec<u8> {
    let img = image::RgbaImage::from_fn(64, 64, |x, _| {
        if x < 32 {
            image::Rgba([255, 0, 0, 255])
        } else {
            image::Rgba([0, 0, 255, 255])
        }
    });
    let mut out = std::io::Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Png).unwrap();
    out.into_inner()
}

fn svg_job(format: vdesigner_core::OutputFormat) -> vdesigner_core::Job {
    vdesigner_core::Job {
        steps: vec![],
        output: vdesigner_core::EncodeSpec {
            format,
            quality: 80,
            lossless: false,
            trace: vdesigner_core::TraceSpec::default(),
        },
    }
}

#[test]
fn svg_preview_is_the_text_the_export_writes() {
    let bytes = two_color_png();
    let job = svg_job(vdesigner_core::OutputFormat::Svg);
    let preview = vdesigner::commands::svg_preview(&bytes, &job, &|| false).unwrap();
    let exported = vdesigner_core::run_job(&bytes, &job, &mut |_| {})
        .unwrap()
        .bytes;
    assert_eq!(preview, String::from_utf8(exported).unwrap());
}

#[test]
fn svg_preview_refuses_a_raster_format() {
    let job = svg_job(vdesigner_core::OutputFormat::WebP);
    let m = vdesigner::commands::svg_preview(&two_color_png(), &job, &|| false).unwrap_err();
    assert!(m.contains("SVG"));
}

#[test]
fn svg_preview_reports_a_superseded_trace_as_cancelled() {
    let job = svg_job(vdesigner_core::OutputFormat::Svg);
    let m = vdesigner::commands::svg_preview(&two_color_png(), &job, &|| true).unwrap_err();
    assert!(m.contains("cancelada"));
}

#[test]
fn the_svg_extension_is_svg() {
    assert_eq!(
        vdesigner::commands::extension_for(vdesigner_core::OutputFormat::Svg),
        "svg"
    );
}
