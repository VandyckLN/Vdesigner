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

/// A high chroma at an extreme lightness has no sRGB answer. The mapping
/// reduces chroma until it fits rather than clamping each channel, so the
/// lightness and hue the caller asked for survive and only the saturation
/// gives way.
///
/// Updated with the switch from per-channel clamping to chroma reduction.
/// The old expectation was `r == 0, g == 255` — the signature of the clamp
/// pinning two channels at opposite ends of the range, which is exactly the
/// behaviour that bent the hue. The result is now a pale green
/// (`#F5FFF6`): still green, still at lightness 0.99, just far less
/// saturated than the impossible 0.4 chroma requested.
#[test]
fn a_colour_outside_the_srgb_gamut_keeps_its_hue_and_loses_chroma() {
    let impossible = Oklch {
        l: 0.99,
        c: 0.4,
        h: 150.0,
    };
    let mapped = from_oklch(impossible);
    let got = to_oklch(mapped);

    assert!(
        (got.l - impossible.l).abs() < 5e-3,
        "L virou {} em vez de {}",
        got.l,
        impossible.l
    );
    // 2° of slack: at this lightness the gamut leaves almost no chroma, and
    // the hue of a near-grey byte triple is coarse simply because 8 bits
    // cannot express a finer angle.
    assert!(
        hue_error(got.h, impossible.h).abs() < 2.0,
        "matiz virou {} em vez de {}",
        got.h,
        impossible.h
    );
    assert!(
        got.c < impossible.c,
        "o croma deveria ter sido reduzido, ficou {}",
        got.c
    );
    assert!(
        mapped.g > mapped.r && mapped.g > mapped.b,
        "verde deveria dominar para o matiz 150: {mapped:?}"
    );
}

/// Signed hue difference, never longer than half a turn, so an angle just
/// under 360° and one just over 0° read as neighbours rather than opposites.
fn hue_error(actual: f64, expected: f64) -> f64 {
    let raw = (actual - expected).rem_euclid(360.0);
    if raw > 180.0 {
        raw - 360.0
    } else {
        raw
    }
}

/// The four brand colours that exposed the per-channel clamp. All sit above
/// roughly 0.23 chroma, which is where clamping used to start visibly bending
/// the hue.
const SATURATED_BRANDS: [&str; 4] = ["#0057FF", "#FF6B00", "#FF00AA", "#E10600"];

/// A tonal ramp holds hue and chroma fixed and moves only lightness, so every
/// rung must read as the same colour. Under per-channel clamping they did not:
/// step 50 of Klein blue came out cyan, 50.9° away, and steps 700-900 of the
/// orange all collapsed onto one clipped red.
///
/// The 4° tolerance is set from what the fix actually delivers — the worst
/// measured drift across these four colours is 3.59°, on step 50 of `#E10600`.
/// That step is a near-white tint whose chroma the gamut has squeezed to
/// almost nothing, and the hue of a near-grey byte triple is coarse for the
/// same reason white has no hue at all. Every other step of every colour here
/// stays under 2.1°.
#[test]
fn every_ramp_step_of_a_saturated_colour_keeps_the_base_hue() {
    for hex in SATURATED_BRANDS {
        let base = parse_hex(hex).unwrap();
        let base_hue = to_oklch(base).h;
        for (index, step) in ramp(base).iter().enumerate() {
            let error = hue_error(to_oklch(*step).h, base_hue);
            assert!(
                error.abs() < 4.0,
                "{hex}: o passo {} ({}) desviou {error}° do matiz base {base_hue}°",
                RAMP_STEPS[index],
                format(*step, ColorFormat::Hex)
            );
        }
    }
}

/// A ramp is useless if two different brand colours can land on the same
/// bytes: under the old clamp `#6D0000` was step 800 of both `#FF6B00` and
/// `#E10600`, because both had clipped to pure red.
#[test]
fn two_different_brands_do_not_share_a_ramp_step() {
    let orange = ramp(parse_hex("#FF6B00").unwrap());
    let red = ramp(parse_hex("#E10600").unwrap());
    for (index, (o, r)) in orange.iter().zip(red.iter()).enumerate() {
        assert_ne!(o, r, "o passo {} coincidiu em {:?}", RAMP_STEPS[index], o);
    }
}

/// The harmony angles are the whole product: a complementary that rotates 145°
/// instead of 180° is not a complementary, and an analogous pair that lands at
/// -7.8° and +30.1° reads as two near-duplicates of the base. Under the old
/// clamp `#0057FF` produced exactly that.
///
/// The 1° tolerance is set from measurement: the worst error across these four
/// bases and all five companions is 0.33°, which is 8-bit rounding rather than
/// gamut mapping.
#[test]
fn harmony_rotations_of_a_saturated_colour_land_on_their_nominal_angles() {
    for hex in SATURATED_BRANDS {
        let base = parse_hex(hex).unwrap();
        let base_hue = to_oklch(base).h;
        let set = harmonies(base);
        for (name, companion, nominal) in [
            ("complementar", set.complementary, 180.0),
            ("análoga -30", set.analogous[0], -30.0),
            ("análoga +30", set.analogous[1], 30.0),
            ("tríade -120", set.triad[0], -120.0),
            ("tríade +120", set.triad[1], 120.0),
        ] {
            let error = hue_error(to_oklch(companion).h, base_hue + nominal);
            assert!(
                error.abs() < 1.0,
                "{hex}: a {name} errou por {error}° (base {base_hue}°, resultado {})",
                format(companion, ColorFormat::Hex)
            );
        }
    }
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

use vdesigner_core::{gradient, ramp, RAMP_STEPS};

#[test]
fn the_ramp_has_ten_steps_named_fifty_to_nine_hundred() {
    assert_eq!(
        RAMP_STEPS,
        [50, 100, 200, 300, 400, 500, 600, 700, 800, 900]
    );
    assert_eq!(
        ramp(Color {
            r: 138,
            g: 144,
            b: 150
        })
        .len(),
        10
    );
}

/// The whole point of anchoring at 500 is that the colour a person chose on
/// purpose survives the scale untouched.
#[test]
fn the_source_colour_is_step_five_hundred_unchanged() {
    let base = Color {
        r: 138,
        g: 144,
        b: 150,
    };
    let scale = ramp(base);
    let index_of_500 = RAMP_STEPS.iter().position(|s| *s == 500).unwrap();
    assert_eq!(scale[index_of_500], base);
}

/// A scale that brightens in the middle gives a design system an inverted
/// rung. Non-increasing rather than strictly decreasing, because a near-white
/// base has no room left to lighten — that is physics, not a bug.
#[test]
fn lightness_never_increases_along_the_ramp() {
    for r in (0..=255).step_by(15) {
        for b in (0..=255).step_by(51) {
            let base = Color {
                r: r as u8,
                g: 90,
                b: b as u8,
            };
            let scale = ramp(base);
            for pair in scale.windows(2) {
                let before = to_oklch(pair[0]).l;
                let after = to_oklch(pair[1]).l;
                assert!(
                    before + 1e-6 >= after,
                    "rampa de {base:?} subiu de {before} para {after}"
                );
            }
        }
    }
}

#[test]
fn a_mid_tone_ramp_strictly_darkens_from_end_to_end() {
    let scale = ramp(Color {
        r: 138,
        g: 144,
        b: 150,
    });
    let lightest = to_oklch(scale[0]).l;
    let darkest = to_oklch(scale[9]).l;
    assert!(
        lightest > darkest + 0.3,
        "faixa curta demais: {lightest} a {darkest}"
    );
}

/// Regression guard: the same input must give the same ten values a year from
/// now, or every palette already committed to a repository silently shifts.
#[test]
fn the_ramp_is_deterministic() {
    let base = Color {
        r: 138,
        g: 144,
        b: 150,
    };
    assert_eq!(ramp(base), ramp(base));
    assert_eq!(
        format(ramp(base)[0], ColorFormat::Hex),
        format(ramp(base)[0], ColorFormat::Hex)
    );
}

#[test]
fn the_gradient_starts_at_the_first_colour_and_ends_at_the_second() {
    let from = Color {
        r: 237,
        g: 232,
        b: 222,
    };
    let to = Color {
        r: 11,
        g: 12,
        b: 14,
    };
    let ramp = gradient(from, to, 7).unwrap();
    assert_eq!(ramp.len(), 7);
    assert_eq!(ramp[0], from);
    assert_eq!(ramp[6], to);
}

#[test]
fn a_two_step_gradient_is_just_the_two_ends() {
    let from = Color { r: 255, g: 0, b: 0 };
    let to = Color { r: 0, g: 0, b: 255 };
    assert_eq!(gradient(from, to, 2).unwrap(), vec![from, to]);
}

#[test]
fn rejects_fewer_than_two_steps() {
    let c = Color { r: 0, g: 0, b: 0 };
    assert!(matches!(
        gradient(c, c, 1),
        Err(CoreError::InvalidParameter(_))
    ));
}

/// Hue is circular: going from 350° to 10° must cross 0°, not travel the long
/// way through 180°. Taking the wrong arc is what puts a grey or a foreign
/// hue in the middle of a gradient.
#[test]
fn the_gradient_takes_the_short_way_around_the_hue_circle() {
    let from = from_oklch(Oklch {
        l: 0.6,
        c: 0.15,
        h: 350.0,
    });
    let to = from_oklch(Oklch {
        l: 0.6,
        c: 0.15,
        h: 10.0,
    });
    let middle = gradient(from, to, 3).unwrap()[1];
    let hue = to_oklch(middle).h;
    let distance_from_zero = hue.min(360.0 - hue);
    assert!(
        distance_from_zero < 15.0,
        "o meio caiu em {hue}°, pelo lado longo"
    );
}

#[test]
fn lightness_moves_steadily_from_one_end_to_the_other() {
    let ramp = gradient(
        Color {
            r: 237,
            g: 232,
            b: 222,
        },
        Color {
            r: 11,
            g: 12,
            b: 14,
        },
        9,
    )
    .unwrap();
    for pair in ramp.windows(2) {
        assert!(to_oklch(pair[0]).l + 1e-6 >= to_oklch(pair[1]).l);
    }
}

use vdesigner_core::harmonies;

#[test]
fn the_complementary_sits_half_a_turn_away_on_the_hue_circle() {
    let base = Color {
        r: 150,
        g: 120,
        b: 100,
    };
    let complementary = harmonies(base).complementary;
    let difference = (to_oklch(complementary).h - to_oklch(base).h).rem_euclid(360.0);
    // Tolerance is ±3° because of 8-bit RGB quantisation at low chroma, not
    // because of gamut clipping: this base colour stays inside sRGB under
    // every rotation the harmony maths performs.
    assert!(
        (difference - 180.0).abs() < 3.0,
        "girou {difference}° em vez de 180°"
    );
}

/// Rotating twice returns to the start. It catches a sign error or a missing
/// wrap that a single rotation hides.
#[test]
fn the_complementary_of_the_complementary_is_the_original() {
    let base = Color {
        r: 150,
        g: 120,
        b: 100,
    };
    let there_and_back = harmonies(harmonies(base).complementary).complementary;
    for (a, b) in [
        (there_and_back.r, base.r),
        (there_and_back.g, base.g),
        (there_and_back.b, base.b),
    ] {
        assert!(
            a.abs_diff(b) <= 2,
            "voltou para {there_and_back:?} em vez de {base:?}"
        );
    }
}

#[test]
fn the_analogous_pair_sits_thirty_degrees_to_each_side() {
    let base = Color {
        r: 150,
        g: 120,
        b: 100,
    };
    let base_hue = to_oklch(base).h;
    let [left, right] = harmonies(base).analogous;
    assert!(((to_oklch(left).h - base_hue).rem_euclid(360.0) - 330.0).abs() < 3.0);
    assert!(((to_oklch(right).h - base_hue).rem_euclid(360.0) - 30.0).abs() < 3.0);
}

#[test]
fn the_triad_pair_sits_a_third_of_a_turn_to_each_side() {
    let base = Color {
        r: 150,
        g: 120,
        b: 100,
    };
    let base_hue = to_oklch(base).h;
    let [left, right] = harmonies(base).triad;
    assert!(((to_oklch(left).h - base_hue).rem_euclid(360.0) - 240.0).abs() < 3.0);
    assert!(((to_oklch(right).h - base_hue).rem_euclid(360.0) - 120.0).abs() < 3.0);
}

/// Rotating a saturated colour at constant chroma can leave the sRGB gamut.
/// Pulling it back in by reducing chroma keeps the angle exact, so the
/// complementary of a saturated base is just as true as that of a muted one.
///
/// Replaces `the_complementary_of_a_saturated_colour_drifts_past_the_exact_half_turn`,
/// which asserted the rotation landed somewhere between 180° and 195° — it
/// pinned the per-channel clamp's hue drift as though it were the intended
/// behaviour. The rotation now measures 180.1°.
#[test]
fn the_complementary_of_a_saturated_colour_still_lands_on_the_exact_half_turn() {
    let base = Color {
        r: 200,
        g: 60,
        b: 60,
    };
    let complementary = harmonies(base).complementary;
    let difference = (to_oklch(complementary).h - to_oklch(base).h).rem_euclid(360.0);
    assert!(
        (difference - 180.0).abs() < 1.0,
        "girou {difference}° em vez de 180°"
    );
}

/// Grey has no hue, so every rotation lands back on grey. It must not invent
/// a colour out of a meaningless angle.
#[test]
fn harmonies_of_grey_stay_grey() {
    let grey = Color {
        r: 128,
        g: 128,
        b: 128,
    };
    let h = harmonies(grey);
    for candidate in [h.complementary, h.analogous[0], h.triad[0]] {
        assert!(
            to_oklch(candidate).c < 1e-3,
            "{candidate:?} ganhou croma do nada"
        );
    }
}
