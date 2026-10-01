use encase::ShaderType;
use glam::Mat4;

/// The fixed look-at view plus perspective projection; 128 bytes.
#[derive(ShaderType, Debug, Clone, Copy, PartialEq)]
pub struct Params {
    pub view: Mat4,
    pub proj: Mat4,
}
