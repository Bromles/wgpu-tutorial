
use encase::ShaderType;
use glam::Vec2;

/// Motion parameters: clip-unit translation and (scale, angle), packed to keep the uniform 16 bytes.
#[derive(ShaderType, Debug, Clone, Copy, PartialEq)]
pub struct Params {
    pub translate: Vec2,
    pub scale_angle: Vec2,
}

impl Params {
    pub fn new(translate: Vec2, scale_angle: Vec2) -> Self {
        Self {
            translate,
            scale_angle,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_PI_2;

    /// The rotation formulas of the chapter, checked by hand first.
    #[test]
    fn rotation_and_translation_match_the_model() {
        let (sin, cos) = (FRAC_PI_2.sin(), FRAC_PI_2.cos());
        // Rotate the direction (0.25, 0) by pi/2: the result is (0, 0.25).
        let x = 0.25_f32;
        let rotated = (x * cos, x * sin);
        assert!((rotated.0 - 0.0).abs() < 1e-6 && (rotated.1 - 0.25).abs() < 1e-6);
        // Translating a point adds the offset; a direction would ignore it.
        let point = (rotated.0 + 0.1, rotated.1 + 0.2);
        assert!((point.0 - 0.1).abs() < 1e-6 && (point.1 - 0.45).abs() < 1e-6);
    }
}
