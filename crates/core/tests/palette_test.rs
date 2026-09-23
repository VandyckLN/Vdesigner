use vdesigner_core::{
    gradient_css, palette_from_json, palette_to_json, validate_palette, CoreError, Generated,
    GradientRef, Palette, Swatch, PALETTE_FORMAT_VERSION,
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

/// Gradients and colours both become `--<nome>` CSS custom properties, so a
/// gradient named the same as a colour would silently overwrite it via the
/// CSS cascade.
#[test]
fn rejects_a_gradient_named_the_same_as_a_colour() {
    let mut palette = sample();
    palette.degrades[0].nome = palette.cores[0].nome.clone();
    let error = validate_palette(&palette).unwrap_err();
    assert!(
        matches!(error, CoreError::InvalidParameter(ref m) if m.contains(&palette.cores[0].nome)),
        "erro pouco claro: {error}"
    );
}

/// A gradient endpoint must resolve against a *colour* name, never against
/// another gradient's name — even though both share one namespace for
/// collision purposes. This distinguishes the `seen` set (all names, for
/// uniqueness) from the `colour_names` set (only colours, for endpoint
/// resolution) inside `validate_palette`.
#[test]
fn rejects_a_gradient_pointing_at_another_gradient_instead_of_a_colour() {
    let mut palette = sample();
    // The second gradient's endpoint names the FIRST gradient, which by then
    // is already present in the uniqueness set — so this only fails if
    // endpoint resolution is kept separate from the uniqueness set.
    let mut segundo = palette.degrades[0].clone();
    segundo.nome = "outro-degrade".into();
    segundo.para = palette.degrades[0].nome.clone();
    palette.degrades.push(segundo);

    let error = validate_palette(&palette).unwrap_err();
    assert!(
        matches!(error, CoreError::InvalidParameter(ref m) if m.contains(&palette.degrades[0].nome)),
        "erro pouco claro: {error}"
    );
}

#[test]
fn rejects_two_gradients_with_the_same_name() {
    let mut palette = sample();
    let mut segundo = palette.degrades[0].clone();
    segundo.nome = palette.degrades[0].nome.clone();
    palette.degrades.push(segundo);
    let error = validate_palette(&palette).unwrap_err();
    assert!(
        matches!(error, CoreError::InvalidParameter(ref m) if m.contains(&palette.degrades[0].nome)),
        "erro pouco claro: {error}"
    );
}

use vdesigner_core::{to_css_vars, to_tailwind};

#[test]
fn a_plain_colour_becomes_one_css_variable() {
    let css = to_css_vars(&sample()).unwrap();
    assert!(
        css.contains("--tinta: #EDE8DE;"),
        "faltou a variável simples:\n{css}"
    );
}

/// The scale is derived at generation time and never stored, so this is the
/// only place the ten rungs become text.
#[test]
fn a_colour_marked_as_ramp_becomes_ten_css_variables() {
    let css = to_css_vars(&sample()).unwrap();
    for step in [50, 100, 200, 300, 400, 500, 600, 700, 800, 900] {
        assert!(
            css.contains(&format!("--acento-{step}:")),
            "faltou --acento-{step}:\n{css}"
        );
    }
}

#[test]
fn a_colour_marked_as_ramp_does_not_also_emit_a_bare_variable() {
    let css = to_css_vars(&sample()).unwrap();
    assert!(
        !css.contains("--acento:"),
        "emitiu --acento: além da escala:\n{css}"
    );
}

#[test]
fn the_ramp_anchor_keeps_the_colour_the_person_chose() {
    let css = to_css_vars(&sample()).unwrap();
    assert!(
        css.contains("--acento-500: #8A9096;"),
        "o 500 não é a cor original:\n{css}"
    );
}

#[test]
fn a_gradient_becomes_a_linear_gradient_variable() {
    let css = to_css_vars(&sample()).unwrap();
    assert!(
        css.contains("--fundo: linear-gradient("),
        "faltou o degradê:\n{css}"
    );
    // Pins the gradient declaration itself (variable name, direction, and the
    // exact `de` colour it starts from), not just the presence of the hex
    // value anywhere in the output — that hex is also emitted by the plain
    // `--tinta:` swatch declaration above, so a bare `contains("#EDE8DE")`
    // would pass even if the gradient pointed the wrong way.
    assert!(
        css.contains("--fundo: linear-gradient(90deg, #EDE8DE,"),
        "o degradê não começa na cor `tinta`:\n{css}"
    );
}

#[test]
fn the_css_is_wrapped_in_a_root_block() {
    let css = to_css_vars(&sample()).unwrap();
    assert!(css.trim_start().starts_with(":root {"));
    assert!(css.trim_end().ends_with('}'));
}

#[test]
fn the_tailwind_fragment_is_valid_json_keyed_by_colour_name() {
    let json = to_tailwind(&sample()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["tinta"], "#EDE8DE");
    assert_eq!(parsed["acento"]["500"], "#8A9096");
}

#[test]
fn it_builds_the_same_gradient_css_the_stylesheet_uses() {
    let palette = Palette {
        versao: PALETTE_FORMAT_VERSION,
        nome: "teste".to_string(),
        gerar: vec![Generated::Css],
        cores: vec![
            Swatch {
                nome: "tinta".to_string(),
                hex: "#ede8de".to_string(),
                rampa: false,
            },
            Swatch {
                nome: "acento".to_string(),
                hex: "#8a9096".to_string(),
                rampa: false,
            },
        ],
        degrades: vec![GradientRef {
            nome: "fundo".to_string(),
            de: "tinta".to_string(),
            para: "acento".to_string(),
        }],
    };

    let css = gradient_css(&palette, &palette.degrades[0]).expect("deve montar o degradê");
    assert!(css.starts_with("linear-gradient(90deg, #"), "saiu: {css}");

    let folha = to_css_vars(&palette).expect("deve montar a folha");
    assert!(
        folha.contains(&std::format!("  --fundo: {css};")),
        "o botão Copiar e o cores.css precisam produzir exatamente a mesma string"
    );
}

#[test]
fn it_refuses_a_gradient_pointing_at_a_colour_that_does_not_exist() {
    let palette = Palette {
        versao: PALETTE_FORMAT_VERSION,
        nome: "teste".to_string(),
        gerar: vec![],
        cores: vec![Swatch {
            nome: "tinta".to_string(),
            hex: "#ede8de".to_string(),
            rampa: false,
        }],
        degrades: vec![],
    };
    let referencia = GradientRef {
        nome: "fundo".to_string(),
        de: "tinta".to_string(),
        para: "fantasma".to_string(),
    };
    assert!(
        gradient_css(&palette, &referencia).is_err(),
        "um degradê apontando para cor inexistente deve falhar antes de gerar CSS"
    );
}
