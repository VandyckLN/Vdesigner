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
