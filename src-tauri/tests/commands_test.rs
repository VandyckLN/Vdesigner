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
