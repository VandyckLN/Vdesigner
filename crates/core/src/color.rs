//! Colour maths. Pure arithmetic — no I/O, no platform, no image decoding.
//!
//! Every ramp, gradient and harmony goes through OKLCH rather than sRGB.
//! Interpolating in sRGB darkens the middle of a gradient between opposite
//! hues and spaces a tonal scale unevenly; OKLCH is perceptually uniform, so
//! equal numeric steps read as equal visual steps.

use crate::error::CoreError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Oklch {
    /// Perceptual lightness, 0.0 (black) to 1.0 (white).
    pub l: f64,
    /// Chroma. Unbounded in theory; sRGB rarely exceeds 0.37.
    pub c: f64,
    /// Hue in degrees, 0.0..360.0. Meaningless when chroma is zero.
    pub h: f64,
}

fn srgb_to_linear(channel: f64) -> f64 {
    if channel <= 0.04045 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(channel: f64) -> f64 {
    if channel <= 0.003_130_8 {
        channel * 12.92
    } else {
        1.055 * channel.powf(1.0 / 2.4) - 0.055
    }
}

pub fn to_oklch(color: Color) -> Oklch {
    let r = srgb_to_linear(f64::from(color.r) / 255.0);
    let g = srgb_to_linear(f64::from(color.g) / 255.0);
    let b = srgb_to_linear(f64::from(color.b) / 255.0);

    // Ottosson's OKLab matrices.
    let l = 0.412_221_470_8 * r + 0.536_332_536_3 * g + 0.051_445_992_9 * b;
    let m = 0.211_903_498_2 * r + 0.680_699_545_1 * g + 0.107_396_956_6 * b;
    let s = 0.088_302_461_9 * r + 0.281_718_837_6 * g + 0.629_978_700_5 * b;

    let l_ = l.cbrt();
    let m_ = m.cbrt();
    let s_ = s.cbrt();

    let lab_l = 0.210_454_255_3 * l_ + 0.793_617_785_0 * m_ - 0.004_072_046_8 * s_;
    let lab_a = 1.977_998_495_1 * l_ - 2.428_592_205_0 * m_ + 0.450_593_709_9 * s_;
    let lab_b = 0.025_904_037_1 * l_ + 0.782_771_766_2 * m_ - 0.808_675_766_0 * s_;

    let chroma = lab_a.hypot(lab_b);
    // atan2(0, 0) is 0 rather than NaN, so grey lands on hue 0 instead of
    // poisoning every later comparison with a NaN.
    let hue = lab_b.atan2(lab_a).to_degrees().rem_euclid(360.0);

    Oklch {
        l: lab_l,
        c: chroma,
        h: hue,
    }
}

/// Linear-light sRGB for an OKLCH triple, before the transfer function and
/// before any clamping. Channels outside [0, 1] here mean the colour is
/// outside the sRGB gamut.
fn linear_rgb_from_oklch(lch: Oklch) -> (f64, f64, f64) {
    let hue = lch.h.to_radians();
    let lab_a = lch.c * hue.cos();
    let lab_b = lch.c * hue.sin();

    let l_ = lch.l + 0.396_337_777_4 * lab_a + 0.215_803_757_3 * lab_b;
    let m_ = lch.l - 0.105_561_345_8 * lab_a - 0.063_854_172_8 * lab_b;
    let s_ = lch.l - 0.089_484_177_5 * lab_a - 1.291_485_548_0 * lab_b;

    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;

    (
        4.076_741_662_1 * l - 3.307_711_591_3 * m + 0.230_969_929_2 * s,
        -1.268_438_004_6 * l + 2.609_757_401_1 * m - 0.341_319_396_5 * s,
        -0.004_196_086_3 * l - 0.703_418_614_7 * m + 1.707_614_701_0 * s,
    )
}

/// Slack on the gamut test. Round-tripping a real sRGB byte lands a channel on
/// 0 or 1 give or take floating-point dust; without the slack those colours
/// would read as out of gamut and get their chroma shaved for nothing.
const GAMUT_EPSILON: f64 = 1e-9;

fn in_srgb_gamut(lch: Oklch) -> bool {
    let (r, g, b) = linear_rgb_from_oklch(lch);
    [r, g, b]
        .iter()
        .all(|c| *c >= -GAMUT_EPSILON && *c <= 1.0 + GAMUT_EPSILON)
}

/// Iterations of the bisection. Each halves the chroma interval, so 16 of them
/// pin the answer to within about 4e-6 of a chroma unit — far finer than the
/// 8-bit channels can express.
const GAMUT_SEARCH_STEPS: u32 = 16;

/// Converts OKLCH to sRGB, mapping out-of-gamut colours back in by **reducing
/// chroma** while holding lightness and hue exactly.
///
/// The obvious alternative — compute the channels and clamp each one into
/// [0, 1] independently — is wrong for a colour tool. Clamping red without
/// touching green and blue changes the *ratio* between the channels, and that
/// ratio is the hue. In practice a saturated blue (#0057FF) lightened for the
/// top of a tonal ramp came out cyan, nearly 51 degrees off, and an orange
/// darkened for the bottom of its ramp clipped to pure red with green and blue
/// flat at zero. The user picked the hue; it is the one property the tool must
/// not silently rewrite. Chroma is negotiable — a slightly duller blue still
/// reads as that blue — so when the requested (L, C, H) has no sRGB answer we
/// bisect the chroma down to the largest value that does, leaving L and H
/// untouched. A colour already inside the gamut is returned unchanged.
pub fn from_oklch(lch: Oklch) -> Color {
    let mapped = if in_srgb_gamut(lch) {
        lch
    } else {
        // Chroma zero is achromatic: in gamut for any lightness in range, so
        // the low end of the bracket is always safe.
        let mut low = 0.0;
        let mut high = lch.c;
        for _ in 0..GAMUT_SEARCH_STEPS {
            let mid = f64::midpoint(low, high);
            if in_srgb_gamut(Oklch { c: mid, ..lch }) {
                low = mid;
            } else {
                high = mid;
            }
        }
        Oklch { c: low, ..lch }
    };

    let (r, g, b) = linear_rgb_from_oklch(mapped);
    Color {
        r: to_byte(linear_to_srgb(r)),
        g: to_byte(linear_to_srgb(g)),
        b: to_byte(linear_to_srgb(b)),
    }
}

/// Clamps before rounding. After the chroma search a channel should only ever
/// sit a hair outside [0, 1] from floating-point error, but an extreme
/// lightness has no in-gamut answer at any chroma, and casting a negative
/// straight to u8 wraps it into a wildly wrong colour.
fn to_byte(channel: f64) -> u8 {
    (channel.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorFormat {
    Hex,
    Rgb,
    Hsl,
    Oklch,
}

pub fn format(color: Color, format: ColorFormat) -> String {
    match format {
        ColorFormat::Hex => std::format!("#{:02X}{:02X}{:02X}", color.r, color.g, color.b),
        ColorFormat::Rgb => std::format!("rgb({}, {}, {})", color.r, color.g, color.b),
        ColorFormat::Hsl => {
            let (h, s, l) = to_hsl(color);
            std::format!(
                "hsl({}, {}%, {}%)",
                h.round(),
                (s * 100.0).round(),
                (l * 100.0).round()
            )
        }
        ColorFormat::Oklch => {
            let lch = to_oklch(color);
            // Chroma of an achromatic colour is a rounding crumb, not a value;
            // printing it would put `oklch(100% 0.0000001 250)` on screen.
            let chroma = if lch.c < 1e-4 { 0.0 } else { lch.c };
            let hue = if chroma == 0.0 { 0.0 } else { lch.h };
            std::format!(
                "oklch({}% {} {})",
                round_to(lch.l * 100.0, 1),
                round_to(chroma, 3),
                round_to(hue, 1)
            )
        }
    }
}

/// Trims a float for display and drops a trailing `.0`, so the common case
/// reads `0` rather than `0.000`.
fn round_to(value: f64, places: u32) -> String {
    let factor = 10_f64.powi(places as i32);
    let rounded = (value * factor).round() / factor;
    if (rounded - rounded.trunc()).abs() < f64::EPSILON {
        std::format!("{}", rounded.trunc() as i64)
    } else {
        std::format!("{rounded}")
    }
}

fn to_hsl(color: Color) -> (f64, f64, f64) {
    let r = f64::from(color.r) / 255.0;
    let g = f64::from(color.g) / 255.0;
    let b = f64::from(color.b) / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let lightness = (max + min) / 2.0;
    let delta = max - min;

    if delta.abs() < f64::EPSILON {
        return (0.0, 0.0, lightness);
    }

    let saturation = delta / (1.0 - (2.0 * lightness - 1.0).abs());
    let hue = if (max - r).abs() < f64::EPSILON {
        60.0 * (((g - b) / delta).rem_euclid(6.0))
    } else if (max - g).abs() < f64::EPSILON {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };

    (hue.rem_euclid(360.0), saturation, lightness)
}

pub fn parse_hex(text: &str) -> Result<Color, CoreError> {
    let digits = text.trim().trim_start_matches('#');

    let expanded = match digits.len() {
        3 => digits.chars().flat_map(|c| [c, c]).collect::<String>(),
        6 => digits.to_string(),
        other => {
            return Err(CoreError::InvalidColor(std::format!(
                "esperado 3 ou 6 dígitos hexadecimais, recebido {other}"
            )))
        }
    };

    // Validate that all characters are ASCII hex digits before slicing to avoid panicking
    // on multi-byte UTF-8 characters at non-character-boundary offsets.
    if !expanded.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(CoreError::InvalidColor(std::format!(
            "`{text}` não é hexadecimal"
        )));
    }

    let channel = |slice: &str| {
        u8::from_str_radix(slice, 16)
            .map_err(|_| CoreError::InvalidColor(std::format!("`{text}` não é hexadecimal")))
    };

    Ok(Color {
        r: channel(&expanded[0..2])?,
        g: channel(&expanded[2..4])?,
        b: channel(&expanded[4..6])?,
    })
}

/// The ten rungs every mainstream design system already speaks. Fixed rather
/// than configurable: inventing a different vocabulary only creates
/// translation work for whoever consumes the generated CSS.
pub const RAMP_STEPS: [u16; 10] = [50, 100, 200, 300, 400, 500, 600, 700, 800, 900];

/// Index of 500 in `RAMP_STEPS`, where the source colour sits.
const ANCHOR: usize = 5;

/// Lightness the top of the scale aims for, and the bottom. Both are pulled
/// back to the base when the base is already past them, which is what keeps a
/// near-white or near-black colour from producing a scale that turns around.
const L_LIGHTEST: f64 = 0.97;
const L_DARKEST: f64 = 0.15;

/// Generates a perceptually uniform ten-step tonal scale from a base colour,
/// with the source colour returned byte-for-byte unchanged at step 500. Hue
/// stays exact across the whole scale; high chroma near white or near black has
/// no sRGB answer, so `from_oklch` shaves the chroma of the extreme steps down
/// to whatever the gamut allows at that lightness. The ends therefore come out
/// less saturated than the arithmetic requests, but never a different colour.
pub fn ramp(base: Color) -> [Color; 10] {
    let anchor = to_oklch(base);
    let lightest = L_LIGHTEST.max(anchor.l);
    let darkest = L_DARKEST.min(anchor.l);

    let mut out = [base; 10];
    for (index, slot) in out.iter_mut().enumerate() {
        if index == ANCHOR {
            // Returned verbatim, not round-tripped, so the anchor is exactly
            // the bytes the person picked rather than the nearest colour the
            // conversion happens to land on.
            continue;
        }
        let lightness = if index < ANCHOR {
            let t = (ANCHOR - index) as f64 / ANCHOR as f64;
            anchor.l + (lightest - anchor.l) * t
        } else {
            let t = (index - ANCHOR) as f64 / (RAMP_STEPS.len() - 1 - ANCHOR) as f64;
            anchor.l + (darkest - anchor.l) * t
        };
        *slot = from_oklch(Oklch {
            l: lightness,
            ..anchor
        });
    }
    out
}

/// Interpolates a gradient between two colours over a given number of steps,
/// returning the endpoints verbatim to ensure exact byte-fidelity to the input.
/// Interpolation happens in OKLCH space: lightness, chroma, and hue all move
/// linearly, but hue wraps around the shortest arc of the circle to avoid
/// dragging the middle through unrelated hues when the endpoints straddle 0°.
pub fn gradient(from: Color, to: Color, steps: usize) -> Result<Vec<Color>, CoreError> {
    if steps < 2 {
        return Err(CoreError::InvalidParameter(
            "um degradê precisa de pelo menos dois passos".into(),
        ));
    }

    let start = to_oklch(from);
    let end = to_oklch(to);
    let hue_delta = shortest_hue_delta(start.h, end.h);

    let mut out = Vec::with_capacity(steps);
    for index in 0..steps {
        if index == 0 {
            out.push(from);
            continue;
        }
        if index == steps - 1 {
            out.push(to);
            continue;
        }
        let t = index as f64 / (steps - 1) as f64;
        out.push(from_oklch(Oklch {
            l: start.l + (end.l - start.l) * t,
            c: start.c + (end.c - start.c) * t,
            h: (start.h + hue_delta * t).rem_euclid(360.0),
        }));
    }
    Ok(out)
}

/// Signed distance around the hue circle, never longer than half a turn.
/// Interpolating the raw difference would walk 340° instead of 20° when the
/// ends straddle 0°, dragging the middle through unrelated hues.
fn shortest_hue_delta(from: f64, to: f64) -> f64 {
    let raw = (to - from).rem_euclid(360.0);
    if raw > 180.0 {
        raw - 360.0
    } else {
        raw
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Harmony {
    pub complementary: Color,
    /// Thirty degrees to each side, in the order [-30°, +30°].
    pub analogous: [Color; 2],
    /// A third of a turn to each side, in the order [-120°, +120°].
    pub triad: [Color; 2],
}

/// Derives four harmonious companion colours from a base colour using the
/// standard five-colour schemes. Complementary is opposite on the hue wheel
/// at 180°; analogous pairs flank the base by 30° on each side; triadic colours
/// sit at thirds of a turn (±120°). Lightness and chroma are preserved across
/// all companions so they read as members of the same family rather than as
/// unrelated colours. A rotation that leaves the sRGB gamut keeps its hue and
/// loses chroma instead, so the angles stay true even for saturated bases.
pub fn harmonies(base: Color) -> Harmony {
    let rotate = |degrees: f64| {
        let lch = to_oklch(base);
        // Lightness and chroma are preserved so the companions read as members
        // of the same family rather than as unrelated colours.
        from_oklch(Oklch {
            h: (lch.h + degrees).rem_euclid(360.0),
            ..lch
        })
    };

    Harmony {
        complementary: rotate(180.0),
        analogous: [rotate(-30.0), rotate(30.0)],
        triad: [rotate(-120.0), rotate(120.0)],
    }
}
