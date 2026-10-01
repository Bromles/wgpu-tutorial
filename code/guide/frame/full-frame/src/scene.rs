use encase::ShaderType;
use glam::{Mat4, Vec3, Vec4};

pub const EYE: Vec3 = Vec3::new(2.6, 3.0, 3.0);
pub const TARGET: Vec3 = Vec3::new(0.0, 0.3, 0.0);
pub const FOV_Y: f32 = std::f32::consts::FRAC_PI_3;
pub const NEAR: f32 = 0.1;
pub const FAR: f32 = 50.0;

/// Fixed direction toward the light; precomputed so Rust and WGSL agree digit for digit.
pub const LIGHT_DIR: Vec3 = Vec3::new(-0.410365, 0.911922, 0.0);

/// Lambert constants carried over from chapter 23; must match the shader constants.
pub const AMBIENT: f32 = 0.1;
pub const INTENSITY: f32 = 0.6;
/// The floor keeps the chapter albedo of 0.5; the cube takes a warm tint.
pub const FLOOR_ALBEDO: Vec3 = Vec3::splat(0.5);
pub const CUBE_ALBEDO: Vec3 = Vec3::new(0.6, 0.45, 0.4);

/// The flat color of the transparent panel: straight alpha, no lighting.
pub const PANEL_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 0.5];

/// Fixed 1024x1024 depth texture, constant receiver bias, no PCF.
pub const SHADOW_MAP_SIZE: u32 = 1024;
/// Must match the shader constant.
pub const SHADOW_BIAS: f32 = 0.0005;

/// A material change is a bind group change, not a data rewrite.
#[derive(ShaderType, Debug, Clone, Copy, PartialEq)]
pub struct Material {
    pub albedo: Vec4,
}

pub const CUBE_MATERIAL: Material = Material {
    albedo: Vec4::new(0.6, 0.45, 0.4, 1.0),
};
pub const FLOOR_MATERIAL: Material = Material {
    albedo: Vec4::new(0.5, 0.5, 0.5, 1.0),
};

/// How many objects the scene holds; also the storage array length.
pub const OBJECT_COUNT: usize = 2;

/// Lifts the cube half a unit so it sits exactly on the floor.
pub const CUBE_TRANSLATION: Vec3 = Vec3::new(0.0, 0.5, 0.0);

/// Two mat4x4s give a 128-byte stride, so records need no padding.
#[derive(ShaderType, Debug, Clone, Copy, PartialEq)]
pub struct ObjectRecord {
    pub model: Mat4,
    /// Inverse transpose of the model's linear part (chapter 24).
    pub normal_matrix: Mat4,
}

/// Derives the normal matrix instead of assuming the identity.
pub fn object_record(translation: Vec3) -> ObjectRecord {
    let model = Mat4::from_translation(translation);
    ObjectRecord {
        model,
        // Linear 3x3 only: translation never applies to a direction.
        normal_matrix: Mat4::from_mat3(glam::Mat3::from_mat4(model).inverse().transpose()),
    }
}

/// The chapter scene table: the lifted cube, then the floor in place.
pub fn objects() -> [ObjectRecord; OBJECT_COUNT] {
    [object_record(CUBE_TRANSLATION), object_record(Vec3::ZERO)]
}

/// The light as an orthographic camera; the light's up maps to world +Z.
pub fn light_view_proj() -> Mat4 {
    glam::camera::rh::proj::directx::orthographic(-2.3, 2.3, -2.3, 2.3, 0.1, 12.0)
        * glam::camera::rh::view::look_at_mat4(LIGHT_DIR * 6.0, Vec3::ZERO, Vec3::Z)
}

/// Mirrors the shader's per-pixel Lambert for the verification crate.
pub fn lambert_linear(albedo: Vec3, diffuse: f32, visible: f32) -> Vec3 {
    albedo * (AMBIENT + INTENSITY * diffuse * visible)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translation_keeps_the_normal_matrix_identity() {
        let record = object_record(CUBE_TRANSLATION);
        assert!(record.normal_matrix.abs_diff_eq(Mat4::IDENTITY, 1e-6));
    }

    // The offscreen test exercises the shadow-footprint probes; nothing to assert here.
}
