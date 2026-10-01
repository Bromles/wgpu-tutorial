/// Linear light in [0, 1] -> sRGB code in [0, 1].
pub fn srgb_encode(linear: f32) -> f32 {
    assert!(
        (0.0..=1.0).contains(&linear),
        "linear value must be in [0, 1]"
    );
    if linear <= 0.003_130_8 {
        12.92 * linear
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    }
}

/// sRGB code in [0, 1] -> linear light.
pub fn srgb_decode(code: f32) -> f32 {
    assert!((0.0..=1.0).contains(&code), "code must be in [0, 1]");
    if code <= 0.040_45 {
        code / 12.92
    } else {
        ((code + 0.055) / 1.055).powf(2.4)
    }
}

/// Quantizes an encoded value to an 8-bit code, rounding half up.
pub fn quantize_u8(encoded: f32) -> u8 {
    (encoded.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// Mean of linear light; `mean_of_codes` averages stored codes instead.
pub fn mean_linear(a: f32, b: f32) -> f32 {
    (a + b) / 2.0
}

pub fn mean_of_codes(a_code: u8, b_code: u8) -> u8 {
    ((u16::from(a_code) + u16::from(b_code)) / 2) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn half_light_is_code_188_not_128() {
        assert_eq!(quantize_u8(srgb_encode(0.5)), 188);
        assert_eq!(quantize_u8(srgb_encode(0.0)), 0);
        assert_eq!(quantize_u8(srgb_encode(1.0)), 255);
    }

    #[test]
    fn decode_is_the_inverse_of_encode() {
        for code in 0..=255u8 {
            let linear = srgb_decode(f32::from(code) / 255.0);
            assert!((0.0..=1.0).contains(&linear));
            assert_eq!(quantize_u8(srgb_encode(linear)), code);
        }
    }

    #[test]
    fn linear_segment_matches_the_piecewise_breakpoints() {
        // The standard's breakpoints keep the curves continuous to ~1e-4, not f32 eps.
        assert!((srgb_encode(0.003_130_8) - 0.040_45).abs() < 1e-4);
        assert!((srgb_decode(0.040_45) - 0.003_130_8).abs() < 1e-4);
        assert!((srgb_encode(0.0) - 0.0).abs() < 1e-9);
    }

    #[test]
    fn mixing_policies_disagree() {
        // Correct: mean light, then encode.
        let correct = quantize_u8(srgb_encode(mean_linear(0.0, 1.0)));
        assert_eq!(correct, 188);
        // Erroneous: mean of stored codes.
        let wrong_code = mean_of_codes(0, 255);
        assert_eq!(wrong_code, 127);
        // Decoded, the wrong code is much less than half the light.
        let wrong_linear = srgb_decode(f32::from(wrong_code) / 255.0);
        assert!((wrong_linear - 0.212).abs() < 0.001);
        assert!(wrong_linear < 0.5);
    }
}
