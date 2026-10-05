//! Uniform structs shared by the frame passes.

use glam::Mat4;

use encase::ShaderType;

/// Camera uniforms of the lit passes; encase rounds the struct to 80 bytes.
#[derive(ShaderType, Debug, Clone, Copy)]
pub(crate) struct PassParams {
    pub(crate) view_proj: Mat4,
    pub(crate) shadows: u32,
}
