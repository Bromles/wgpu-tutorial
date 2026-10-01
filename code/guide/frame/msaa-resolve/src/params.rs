use encase::ShaderType;
use glam::Mat4;

/// Fixed view plus resize-recomputed projection; 128 bytes.
#[derive(ShaderType, Debug, Clone, Copy, PartialEq)]
pub struct Params {
    pub view: Mat4,
    pub proj: Mat4,
}
