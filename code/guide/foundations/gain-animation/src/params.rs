use encase::ShaderType;
use glam::Vec4;

/// Per-draw parameters. Each triangle gets its own uniform buffer and bind
/// group, so both values live simultaneously in one submission.
#[derive(ShaderType, Debug, Clone, Copy, PartialEq)]
pub struct Params {
    pub tint: Vec4,
    pub gain: f32,
}

impl Params {
    pub fn with_gain(gain: f32) -> Self {
        Self {
            tint: Vec4::ONE,
            gain,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use encase::UniformBuffer;

    #[test]
    fn gain_formula_matches_the_passport() {
        let speed = 0.25;
        // gain = min(v * t, 1): the exercise checkpoints of the chapter.
        assert_eq!((speed * 2.0_f32).min(1.0), 0.5, "checkpoint t=2: gain is halfway");
        assert_eq!((speed * 8.0_f32).min(1.0), 1.0, "checkpoint t=8: gain saturates at 1");
        assert_eq!((speed * 0.0_f32).min(1.0), 0.0, "checkpoint t=0: gain starts at 0");
    }

    #[test]
    fn params_serializes_to_32_bytes() {
        let mut buffer = UniformBuffer::new(Vec::<u8>::new());
        buffer.write(&Params::with_gain(0.5)).unwrap();
        assert_eq!(buffer.into_inner().len(), 32);
    }
}
