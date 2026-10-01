use encase::ShaderType;
use glam::Mat4;

/// Ortho design frame half extents; preset B of chapter 27a fills the frame.
pub const HALF_X: f32 = 2.4;
pub const HALF_Y: f32 = 1.8;

/// Background sits half a unit behind the sources so the depth test separates them.
pub const Z_BACKGROUND: f32 = -1.5;
pub const Z_SOURCES: f32 = -1.0;

/// Near/far land Z_BACKGROUND at NDC depth 0.75 and Z_SOURCES at 0.5.
pub const NEAR: f32 = 0.0;
pub const FAR: f32 = 2.0;

/// Fixed ortho camera, eye at the origin looking along -Z (chapter 19a).
pub fn ortho() -> Mat4 {
    glam::camera::rh::proj::directx::orthographic(-HALF_X, HALF_X, -HALF_Y, HALF_Y, NEAR, FAR)
}

/// NDC depth of a world z under the fixed ortho volume.
pub fn depth(z: f32) -> f32 {
    (-z - NEAR) / (FAR - NEAR)
}

/// The only uniform of the snapshot; static, so it is uploaded once.
#[derive(ShaderType, Debug, Clone, Copy, PartialEq)]
pub struct Params {
    pub view_proj: Mat4,
}

/// Preset B colors: opaque black background, red/green sources at alpha 0.5.
pub const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
pub const RED: [f32; 4] = [1.0, 0.0, 0.0, 0.5];
pub const GREEN: [f32; 4] = [0.0, 1.0, 0.0, 0.5];

/// Cutout alphas ride just below/above the discard threshold.
pub const CUT_BELOW: f32 = 0.49;
pub const CUT_ABOVE: f32 = 0.51;
pub const CUTOUT_THRESHOLD: f32 = 0.5;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sources_sit_in_front_of_the_background() {
        assert!((depth(Z_SOURCES) - 0.5).abs() < 1e-6);
        assert!((depth(Z_BACKGROUND) - 0.75).abs() < 1e-6);
        assert!(depth(Z_SOURCES) < depth(Z_BACKGROUND));
    }

    #[test]
    fn cutout_alphas_bracket_the_threshold() {
        const { assert!(CUT_BELOW < CUTOUT_THRESHOLD) };
        const { assert!(CUT_ABOVE > CUTOUT_THRESHOLD) };
    }
}
