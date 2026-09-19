use vdesigner_core::{from_oklch, to_oklch, Color, Oklch};

/// Sweeps the cube instead of testing a handful of colours, because a wrong
/// matrix coefficient can be invisible on primaries and obvious on mixes.
#[test]
fn srgb_to_oklch_and_back_returns_the_same_colour() {
    for r in (0..=255).step_by(17) {
        for g in (0..=255).step_by(51) {
            for b in (0..=255).step_by(85) {
                let original = Color {
                    r: r as u8,
                    g: g as u8,
                    b: b as u8,
                };
                let round_trip = from_oklch(to_oklch(original));
                assert_eq!(
                    round_trip, original,
                    "ida e volta mudou a cor {original:?} para {round_trip:?}"
                );
            }
        }
    }
}

#[test]
fn white_has_lightness_one_and_no_chroma() {
    let lch = to_oklch(Color {
        r: 255,
        g: 255,
        b: 255,
    });
    assert!((lch.l - 1.0).abs() < 1e-3, "L de branco foi {}", lch.l);
    assert!(lch.c < 1e-3, "croma de branco foi {}", lch.c);
}

#[test]
fn black_has_zero_lightness_and_no_chroma() {
    let lch = to_oklch(Color { r: 0, g: 0, b: 0 });
    assert!(lch.l.abs() < 1e-3, "L de preto foi {}", lch.l);
    assert!(lch.c < 1e-3, "croma de preto foi {}", lch.c);
}

/// Grey has no hue to report. The conversion must settle on a finite number
/// rather than emit NaN from atan2 of two zeros — every later step divides,
/// compares or interpolates this value.
#[test]
fn pure_grey_reports_a_finite_hue() {
    let lch = to_oklch(Color {
        r: 128,
        g: 128,
        b: 128,
    });
    assert!(lch.h.is_finite(), "matiz de cinza puro foi {}", lch.h);
    assert!(lch.c < 1e-3);
}

/// A high chroma at an extreme lightness has no sRGB answer: the linear RGB
/// this hue and chroma imply pushes red below 0 and green above 1. Clamping
/// before rounding keeps every channel a real sRGB byte instead of letting a
/// negative value wrap around a `u8` cast into a bogus colour, and the hue
/// the input asked for (150 degrees, green) still dominates the result.
#[test]
fn colour_outside_the_srgb_gamut_is_clipped_into_range() {
    let impossible = Oklch {
        l: 0.99,
        c: 0.4,
        h: 150.0,
    };
    let clipped = from_oklch(impossible);

    // Clamping the out-of-range channels pins red at 0 and green at the
    // gamut ceiling; a wraparound bug would instead surface as a channel
    // far from either boundary.
    assert_eq!(clipped.r, 0, "vermelho ficou {}", clipped.r);
    assert_eq!(clipped.g, 255, "verde ficou {}", clipped.g);
    assert!(
        clipped.g > clipped.r && clipped.g > clipped.b,
        "verde deveria dominar para o matiz 150: {clipped:?}"
    );
}

use vdesigner_core::{format, parse_hex, ColorFormat, CoreError};

#[test]
fn formats_hex_in_upper_case_with_a_leading_hash() {
    let c = Color {
        r: 138,
        g: 144,
        b: 150,
    };
    assert_eq!(format(c, ColorFormat::Hex), "#8A9096");
}

#[test]
fn formats_rgb_in_the_css_function_form() {
    let c = Color {
        r: 138,
        g: 144,
        b: 150,
    };
    assert_eq!(format(c, ColorFormat::Rgb), "rgb(138, 144, 150)");
}

#[test]
fn formats_oklch_in_the_css_function_form() {
    let c = Color {
        r: 255,
        g: 255,
        b: 255,
    };
    assert_eq!(format(c, ColorFormat::Oklch), "oklch(100% 0 0)");
}

#[test]
fn parses_six_digit_hex_with_or_without_the_hash() {
    let expected = Color {
        r: 138,
        g: 144,
        b: 150,
    };
    assert_eq!(parse_hex("#8A9096").unwrap(), expected);
    assert_eq!(parse_hex("8a9096").unwrap(), expected);
}

#[test]
fn parses_three_digit_hex_by_doubling_each_digit() {
    assert_eq!(
        parse_hex("#0F8").unwrap(),
        Color {
            r: 0,
            g: 255,
            b: 136
        }
    );
}

#[test]
fn rejects_hex_of_the_wrong_length() {
    assert!(matches!(
        parse_hex("#12345"),
        Err(CoreError::InvalidColor(_))
    ));
}

#[test]
fn rejects_hex_with_a_non_hex_digit() {
    assert!(matches!(
        parse_hex("#8A90ZZ"),
        Err(CoreError::InvalidColor(_))
    ));
}

/// The hex text is what a person types and what the JSON file stores, so a
/// value that survives formatting must survive reading back unchanged.
#[test]
fn hex_round_trips_through_format_and_parse() {
    for r in (0..=255).step_by(37) {
        let c = Color {
            r: r as u8,
            g: 17,
            b: 240,
        };
        assert_eq!(parse_hex(&format(c, ColorFormat::Hex)).unwrap(), c);
    }
}

/// Non-ASCII hex input must return an error, not panic on multi-byte UTF-8
/// character boundaries. This validates that the parser checks for ASCII hex
/// digits before ever slicing.
#[test]
fn rejects_non_ascii_hex_without_panicking() {
    // 6-byte string with multi-byte UTF-8 character (é) at a boundary where
    // slicing without validation would panic. This must return Err, not panic.
    assert!(matches!(
        parse_hex("a\u{e9}bcd"),
        Err(CoreError::InvalidColor(_))
    ));

    // Different byte layout: multi-byte character at the start.
    assert!(matches!(
        parse_hex("\u{e9}abcdef"),
        Err(CoreError::InvalidColor(_))
    ));
}
