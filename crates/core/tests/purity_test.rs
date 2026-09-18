/// The engine must never gain a UI or platform dependency. This test reads the
/// manifest as text, which is enough to catch an accidental addition in review.
#[test]
fn core_manifest_has_no_ui_or_platform_dependencies() {
    let manifest = include_str!("../Cargo.toml");
    for forbidden in ["tauri", "windows", "winapi", "wry"] {
        assert!(
            !manifest.contains(forbidden),
            "vdesigner-core não pode depender de `{forbidden}`"
        );
    }
}
