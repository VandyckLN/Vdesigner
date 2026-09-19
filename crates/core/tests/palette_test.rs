use vdesigner_core::{
    palette_from_json, palette_to_json, validate_palette, CoreError, Generated, GradientRef,
    Palette, Swatch, PALETTE_FORMAT_VERSION,
};

fn sample() -> Palette {
    Palette {
        versao: PALETTE_FORMAT_VERSION,
        nome: "Vdesigner".into(),
        gerar: vec![Generated::Css],
        cores: vec![
            Swatch {
                nome: "tinta".into(),
                hex: "#EDE8DE".into(),
                rampa: false,
            },
            Swatch {
                nome: "acento".into(),
                hex: "#8A9096".into(),
                rampa: true,
            },
        ],
        degrades: vec![GradientRef {
            nome: "fundo".into(),
            de: "tinta".into(),
            para: "acento".into(),
        }],
    }
}

/// The chosen folder is the source of truth, so the file has to survive a full
/// trip without losing a field or reordering the colours.
#[test]
fn a_palette_survives_a_trip_through_json() {
    let original = sample();
    let text = palette_to_json(&original).unwrap();
    let back = palette_from_json(&text).unwrap();
    assert_eq!(back, original);
}

#[test]
fn reading_a_future_format_version_fails_loudly() {
    let mut future = sample();
    future.versao = PALETTE_FORMAT_VERSION + 1;
    let text = serde_json::to_string(&future).unwrap();
    let error = palette_from_json(&text).unwrap_err();
    assert!(
        matches!(error, CoreError::InvalidParameter(ref m) if m.contains("versão")),
        "erro pouco claro: {error}"
    );
}

#[test]
fn rejects_a_colour_name_with_a_space() {
    let mut palette = sample();
    palette.cores[0].nome = "cor principal".into();
    assert!(matches!(
        validate_palette(&palette),
        Err(CoreError::InvalidParameter(_))
    ));
}

#[test]
fn rejects_a_colour_name_with_an_accent_or_upper_case() {
    for bad in ["Acento", "acentuação"] {
        let mut palette = sample();
        palette.cores[0].nome = bad.into();
        assert!(
            matches!(
                validate_palette(&palette),
                Err(CoreError::InvalidParameter(_))
            ),
            "`{bad}` deveria ser recusado"
        );
    }
}

#[test]
fn accepts_a_kebab_case_name() {
    let mut palette = sample();
    // The gradient reference is updated along with the name so this exercises
    // only kebab-case acceptance, not a dangling-reference rejection.
    palette.cores[0].nome = "cor-principal".into();
    palette.degrades[0].de = "cor-principal".into();
    assert!(validate_palette(&palette).is_ok());
}

#[test]
fn rejects_two_colours_with_the_same_name() {
    let mut palette = sample();
    palette.cores[1].nome = palette.cores[0].nome.clone();
    assert!(matches!(
        validate_palette(&palette),
        Err(CoreError::InvalidParameter(_))
    ));
}

#[test]
fn rejects_an_invalid_hex_value() {
    let mut palette = sample();
    palette.cores[0].hex = "#ZZZ".into();
    assert!(matches!(
        validate_palette(&palette),
        Err(CoreError::InvalidColor(_))
    ));
}

/// Gradients point at colours by name so that editing a colour updates them.
/// A dangling name means the file is already inconsistent.
#[test]
fn rejects_a_gradient_pointing_at_a_colour_that_does_not_exist() {
    let mut palette = sample();
    palette.degrades[0].para = "inexistente".into();
    let error = validate_palette(&palette).unwrap_err();
    assert!(matches!(error, CoreError::InvalidParameter(ref m) if m.contains("inexistente")));
}
