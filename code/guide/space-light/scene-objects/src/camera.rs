use encase::ShaderType;
use glam::{Mat4, Vec3, Vec4};

/// Fixed pose: a simplified chapter 21 camera, above the plane, aimed at the origin.
pub const EYE: Vec3 = Vec3::new(0.0, 0.6, 3.0);
pub const TARGET: Vec3 = Vec3::ZERO;
pub const FOV_Y: f32 = std::f32::consts::FRAC_PI_3;
pub const NEAR: f32 = 0.1;
pub const FAR: f32 = 50.0;

/// View: the world expressed in camera coordinates (chapter 18).
pub fn view() -> Mat4 {
    glam::camera::rh::view::look_at_mat4(EYE, TARGET, Vec3::Y)
}

/// Aspect-dependent perspective of chapter 19a.
pub fn projection(aspect: f32) -> Mat4 {
    glam::camera::rh::proj::directx::perspective(FOV_Y, aspect, NEAR, FAR)
}

/// Full chain P·V: a model matrix lands on the right, so a vertex travels M, V, then P.
pub fn view_proj(aspect: f32) -> Mat4 {
    projection(aspect) * view()
}

/// Group 0 per-frame uniform: camera chain plus the eye for the specular term; 80 bytes.
#[derive(ShaderType, Debug, Clone, Copy, PartialEq)]
pub struct CameraParams {
    pub view_proj: Mat4,
    pub eye: Vec4,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The camera keeps looking at the origin: dead centre of the frame.
    #[test]
    fn the_origin_sits_dead_centre() {
        let clip = view_proj(4.0 / 3.0) * Vec4::new(0.0, 0.0, 0.0, 1.0);
        let ndc = clip.truncate() / clip.w;
        assert!(ndc.x.abs() < 1e-6 && ndc.y.abs() < 1e-6);
        assert!(ndc.z > 0.0 && ndc.z < 1.0);
    }
}
