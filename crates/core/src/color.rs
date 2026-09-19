//! Colour maths. Pure arithmetic — no I/O, no platform, no image decoding.
//!
//! Every ramp, gradient and harmony goes through OKLCH rather than sRGB.
//! Interpolating in sRGB darkens the middle of a gradient between opposite
//! hues and spaces a tonal scale unevenly; OKLCH is perceptually uniform, so
//! equal numeric steps read as equal visual steps.

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

pub fn from_oklch(lch: Oklch) -> Color {
    let hue = lch.h.to_radians();
    let lab_a = lch.c * hue.cos();
    let lab_b = lch.c * hue.sin();

    let l_ = lch.l + 0.396_337_777_4 * lab_a + 0.215_803_757_3 * lab_b;
    let m_ = lch.l - 0.105_561_345_8 * lab_a - 0.063_854_172_8 * lab_b;
    let s_ = lch.l - 0.089_484_177_5 * lab_a - 1.291_485_548_0 * lab_b;

    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;

    let r = 4.076_741_662_1 * l - 3.307_711_591_3 * m + 0.230_969_929_2 * s;
    let g = -1.268_438_004_6 * l + 2.609_757_401_1 * m - 0.341_319_396_5 * s;
    let b = -0.004_196_086_3 * l - 0.703_418_614_7 * m + 1.707_614_701_0 * s;

    Color {
        r: to_byte(linear_to_srgb(r)),
        g: to_byte(linear_to_srgb(g)),
        b: to_byte(linear_to_srgb(b)),
    }
}

/// Clamps before rounding. A colour outside the sRGB gamut produces a channel
/// below 0 or above 1, and casting that straight to u8 wraps it into a wildly
/// wrong colour instead of the nearest real one.
fn to_byte(channel: f64) -> u8 {
    (channel.clamp(0.0, 1.0) * 255.0).round() as u8
}
