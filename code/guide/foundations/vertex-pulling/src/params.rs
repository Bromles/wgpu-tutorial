use encase::ShaderType;
use glam::Vec4;

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
