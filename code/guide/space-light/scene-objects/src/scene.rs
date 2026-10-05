use encase::ShaderType;
use glam::Mat3;
use glam::{Mat4, Vec3, Vec4};

/// Reflection parameters of the surface, not of the light or viewer.
/// Shininess stays a shader constant, keeping the two-vec4 layout (32 bytes, no padding).
#[derive(ShaderType, Debug, Clone, Copy, PartialEq)]
pub struct Material {
    pub albedo: Vec4,
    pub specular: Vec4,
}

/// Chapter materials: warm red left, cool blue right; same specular strength.
pub const WARM: Material = Material {
    albedo: Vec4::new(0.70, 0.25, 0.20, 1.0),
    specular: Vec4::new(0.5, 0.5, 0.5, 1.0),
};
pub const COOL: Material = Material {
    albedo: Vec4::new(0.20, 0.35, 0.70, 1.0),
    specular: Vec4::new(0.5, 0.5, 0.5, 1.0),
};

/// How many objects the scene holds; also the storage array length.
pub const OBJECT_COUNT: usize = 2;

/// Object 0 keeps the identity (mesh authored on the left); object 1 reuses it, X moves it to the second pose.
pub const RIGHT_HOME: Vec3 = Vec3::new(1.8, 0.0, 0.0);
pub const RIGHT_MOVED: Vec3 = Vec3::new(2.4, 0.8, 0.0);

/// One object: shared geometry (one mesh) plus its transform pair; 128-byte stride, no padding.
#[derive(ShaderType, Debug, Clone, Copy, PartialEq)]
pub struct ObjectRecord {
    pub model: Mat4,
    /// Inverse transpose of the model's linear part (chapter 24); identity for a pure translation.
    pub normal_matrix: Mat4,
}

/// Builds a translated object's record, deriving the normal matrix the honest way.
pub fn object_record(translation: Vec3) -> ObjectRecord {
    let model = Mat4::from_translation(translation);
    ObjectRecord {
        model,
        // Linear 3x3 part only (chapter 24): translation never applies to a direction.
        normal_matrix: Mat4::from_mat3(Mat3::from_mat4(model).inverse().transpose()),
    }
}

/// Chapter scene table: identity left, translation right.
pub fn objects() -> [ObjectRecord; OBJECT_COUNT] {
    [object_record(Vec3::ZERO), object_record(RIGHT_HOME)]
}

/// Chapter 25 directional light simplified to constants: the chapter is about data roles, not shading.
/// Must match the shader constants.
pub const LIGHT_DIR: Vec3 = Vec3::Z;
pub const AMBIENT: f32 = 0.1;
pub const INTENSITY: f32 = 0.6;
pub const SHININESS: f32 = 32.0;

/// Lambert plus the Blinn-Phong highlight: the shader's model mirrored on the CPU.
pub fn shade(material: Material, p: Vec3, n: Vec3, eye: Vec3) -> Vec3 {
    let v = (eye - p).normalize();
    let diffuse = n.dot(LIGHT_DIR).max(0.0);
    let l_plus_v = LIGHT_DIR + v;
    let mut specular = 0.0;
    if l_plus_v.length() > 0.0 {
        let h = l_plus_v.normalize();
        specular = n.dot(h).clamp(0.0, 1.0).powf(SHININESS);
    }
    material.albedo.truncate() * (AMBIENT + INTENSITY * diffuse)
        + material.specular.truncate() * (INTENSITY * specular)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera;

    /// Translation leaves the normal matrix identity: moving does not turn surfaces.
    #[test]
    fn translation_keeps_the_normal_matrix_identity() {
        let record = object_record(RIGHT_HOME);
        assert!(record.normal_matrix.abs_diff_eq(Mat4::IDENTITY, 1e-6));
    }

    /// Mirror points for the on-axis camera shade identically under one material.
    #[test]
    fn shared_material_shades_mirror_points_equally() {
        let left = shade(WARM, Vec3::new(-0.9, 0.0, 0.0), Vec3::Z, camera::EYE);
        let right = shade(WARM, Vec3::new(0.9, 0.0, 0.0), Vec3::Z, camera::EYE);
        assert!((left - right).length() < 1e-6);
    }

    /// The two materials really differ where it shows: in the albedo.
    #[test]
    fn distinct_materials_shade_differently() {
        let p = Vec3::new(-0.9, 0.0, 0.0);
        let warm = shade(WARM, p, Vec3::Z, camera::EYE);
        let cool = shade(COOL, p, Vec3::Z, camera::EYE);
        assert!((warm - cool).length() > 0.1);
    }
}
