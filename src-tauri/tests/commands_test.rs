use std::fs;
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
