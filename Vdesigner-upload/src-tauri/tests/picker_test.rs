use vdesigner::picker::{Picker, PickerError};
use vdesigner::screen::{ScreenError, Snapshot};

fn snapshot_2x1() -> Snapshot {
    // One red pixel and one blue pixel, side by side, origin at zero.
    let pixels = vec![255, 0, 0, 255, 0, 0, 255, 255];
    Snapshot::from_raw(0, 0, 2, 1, pixels, Vec::new()).expect("bitmap sintético válido")
}

#[test]
fn it_refuses_to_resolve_before_being_armed() {
    let picker = Picker::default();
    assert!(
        matches!(picker.resolve(0, 0), Err(PickerError::NotArmed)),
        "escolher sem retrato deve falhar, não devolver uma cor inventada"
    );
}

#[test]
fn it_resolves_the_pixel_under_the_cursor_as_hex() {
    let picker = Picker::default();
    picker.arm(snapshot_2x1()).expect("armar deve funcionar");
    // Uppercase because that is the single hex spelling the core emits
    // (`ColorFormat::Hex`) and the palette already stores; a lowercase
    // spelling here would give the same colour two names in one app.
    assert_eq!(picker.resolve(0, 0).expect("deve resolver"), "#FF0000");
    assert_eq!(picker.resolve(1, 0).expect("deve resolver"), "#0000FF");
}

#[test]
fn it_reports_the_geometry_the_overlay_needs() {
    let picker = Picker::default();
    let geometry = picker.arm(snapshot_2x1()).expect("armar deve funcionar");
    assert_eq!((geometry.width, geometry.height), (2, 1));
    assert!(
        !geometry.png_base64.is_empty(),
        "a sobreposição não tem o que mostrar sem o retrato"
    );
}

#[test]
fn it_forgets_the_snapshot_when_disarmed() {
    let picker = Picker::default();
    picker.arm(snapshot_2x1()).expect("armar deve funcionar");
    picker.disarm();
    assert!(
        matches!(picker.resolve(0, 0), Err(PickerError::NotArmed)),
        "um retrato de 4K vazaria memória se sobrevivesse ao fechamento"
    );
}

#[test]
fn it_resolves_an_overlay_click_through_the_window_dpi_factor() {
    // The overlay reports CSS pixels. At 200% the second physical pixel of a
    // 2x1 snapshot is only reachable at CSS x = 0.5, so a picker that
    // ignored the factor would return red for both halves.
    let picker = Picker::default();
    picker.arm(snapshot_2x1()).expect("armar deve funcionar");
    assert_eq!(
        picker
            .resolve_overlay_point(0.0, 0.0, 2.0)
            .expect("deve resolver"),
        "#FF0000"
    );
    assert_eq!(
        picker
            .resolve_overlay_point(0.5, 0.0, 2.0)
            .expect("deve resolver"),
        "#0000FF",
        "meio pixel CSS a 200% já é o segundo pixel físico"
    );
}

#[test]
fn it_refuses_an_overlay_click_before_being_armed() {
    let picker = Picker::default();
    assert!(
        matches!(
            picker.resolve_overlay_point(0.0, 0.0, 1.0),
            Err(PickerError::NotArmed)
        ),
        "um clique sem retrato deve falhar, não devolver uma cor inventada"
    );
}

#[test]
fn it_refuses_a_position_outside_the_snapshot() {
    let picker = Picker::default();
    picker.arm(snapshot_2x1()).expect("armar deve funcionar");
    assert!(
        picker.resolve(9, 9).is_err(),
        "uma posição fora do retrato deve falhar em vez de ler lixo"
    );
}

#[test]
fn it_replaces_the_previous_snapshot_when_armed_again() {
    let picker = Picker::default();
    picker
        .arm(snapshot_2x1())
        .expect("primeiro arm deve funcionar");
    assert_eq!(picker.resolve(0, 0).expect("deve resolver"), "#FF0000");

    let green_pixels = vec![0, 255, 0, 255];
    let second_snapshot =
        Snapshot::from_raw(0, 0, 1, 1, green_pixels, Vec::new()).expect("segundo snapshot válido");
    picker
        .arm(second_snapshot)
        .expect("segundo arm deve funcionar");

    assert_eq!(
        picker.resolve(0, 0).expect("deve resolver"),
        "#00FF00",
        "o segundo arm deve substituir o retrato anterior"
    );
    assert!(
        picker.resolve(1, 0).is_err(),
        "as dimensões do retrato anterior não devem ser acessíveis após novo arm"
    );
}

#[test]
fn it_refuses_invalid_overlay_coordinates() {
    let picker = Picker::default();
    picker.arm(snapshot_2x1()).expect("armar deve funcionar");
    assert!(
        matches!(
            picker.resolve_overlay_point(f64::NAN, 0.0, 1.0),
            Err(PickerError::Screen(ScreenError::InvalidPoint))
        ),
        "coordenadas NaN devem resultar em erro"
    );
}

#[test]
fn it_reports_current_geometry_when_armed() {
    let picker = Picker::default();
    assert!(picker.current_geometry().is_none());
    let geometry = picker.arm(snapshot_2x1()).expect("armar deve funcionar");
    let current = picker.current_geometry().expect("deve ter geometria atual");
    assert_eq!(
        (current.width, current.height),
        (geometry.width, geometry.height)
    );
    assert_eq!(current.png_base64, geometry.png_base64);
}

#[test]
fn it_returns_none_for_current_geometry_when_disarmed() {
    let picker = Picker::default();
    picker.arm(snapshot_2x1()).expect("armar deve funcionar");
    picker.disarm();
    assert!(picker.current_geometry().is_none());
}
