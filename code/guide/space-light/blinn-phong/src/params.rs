use encase::ShaderType;
use glam::{Mat4, Vec3};

/// Surface reflection parameters: albedo tints diffuse, specular is the highlight color, shininess narrows it.
#[derive(ShaderType, Debug, Clone, Copy, PartialEq)]
pub struct Material {
    pub albedo: Vec3,
    pub specular: Vec3,
    pub shininess: f32,
}

/// Per-frame uniform; encase's 16-byte vec3 alignment makes it 128 bytes.
#[derive(ShaderType, Debug, Clone, Copy, PartialEq)]
pub struct Params {
    pub view_proj: Mat4,
    /// World-space position of the eye; V = normalize(eye - P).
    pub eye: Vec3,
    /// Unit direction from the surface toward the light.
    pub light_dir: Vec3,
    pub material: Material,
}

impl Material {
    /// The chapter material: matte grey base with a strong highlight.
    pub const CHAPTER: Material = Material {
        albedo: Vec3::splat(0.5),
        specular: Vec3::splat(0.7),
        shininess: 32.0,
    };
}

impl Params {
    pub fn new(view_proj: Mat4, eye: Vec3, light_dir: Vec3, material: Material) -> Self {
        Self {
            view_proj,
            eye,
            light_dir,
            material,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use encase::UniformBuffer;

    /// Byte layout: matrix at 0, vec3s 16-aligned (eye 64, light_dir 80), Material at 96, size 128.
    #[test]
    fn params_layout_matches_wgsl_contract() {
        let params = Params::new(
            Mat4::IDENTITY,
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(4.0, 5.0, 6.0),
            Material {
                albedo: Vec3::new(7.0, 8.0, 9.0),
                specular: Vec3::new(10.0, 11.0, 12.0),
                shininess: 13.0,
            },
        );
        let mut buffer = UniformBuffer::new(Vec::<u8>::new());
        buffer.write(&params).unwrap();
        let bytes = buffer.into_inner();
        assert_eq!(bytes.len(), 128, "struct size rounds up to alignment 16");
        // 1.0 at offsets 64, 80 and 96 (three vec3 starts), 13.0 at 124.
        for (offset, value) in [(64, 1.0f32), (80, 4.0), (96, 7.0), (124, 13.0)] {
            let mut probe = [0u8; 4];
            probe.copy_from_slice(&bytes[offset..offset + 4]);
            assert_eq!(f32::from_le_bytes(probe), value, "offset {offset}");
        }
        // The gaps are padding: e.g. the 4 bytes after each vec3.
        assert!(bytes[76..80].iter().all(|&b| b == 0));
        assert!(bytes[92..96].iter().all(|&b| b == 0));
    }

    /// V = L at the centre, so H = +Z and the specular factor is exactly 1.
    #[test]
    fn half_vector_at_the_centre_points_at_the_light() {
        let eye = Vec3::new(0.0, 0.0, 3.0);
        let v = (eye - Vec3::ZERO).normalize();
        let h = (Vec3::Z + v).normalize();
        assert!((h - Vec3::Z).length() < 1e-6);
        assert!((Vec3::Z.dot(h) - 1.0).abs() < 1e-6);
    }

    /// N*H = 0.8 with exponents 8 and 32 gives ~0.168 and ~0.000792.
    #[test]
    fn shininess_narrows_the_highlight() {
        assert!((0.8_f32.powf(8.0) - 0.16777).abs() < 1e-4);
        assert!((0.8_f32.powf(32.0) - 0.000792).abs() < 1e-6);
    }

    /// V exactly against L leaves no half vector: the sum is degenerate.
    #[test]
    fn degenerate_half_vector_is_rejected() {
        let l = Vec3::Z;
        let v = -Vec3::Z;
        assert!((l + v).length_squared() < 1e-6);
    }
}
