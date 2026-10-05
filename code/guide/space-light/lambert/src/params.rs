use encase::ShaderType;
use glam::Vec3;

/// Directional light and material response, all in linear units.
/// vec3s align to 16, scalars to 4: light_dir 0, albedo 16, scalars 28/32/36, size 48.
#[derive(ShaderType, Debug, Clone, Copy)]
pub struct LightParams {
    /// Unit direction from the surface toward the light source.
    pub light_dir: Vec3,
    /// Base surface color (fraction of light the material reflects).
    pub albedo: Vec3,
    /// Constant term: an approximation of indirect light, not a computation of it.
    pub ambient: f32,
    /// Strength of the direct (Lambert) term.
    pub intensity: f32,
    /// 1 = show 0.5*(N+1) instead of shading; the N key switches this.
    pub show_normals: u32,
}

impl LightParams {
    /// Chapter start values; the frame opens in normal-as-color mode.
    pub fn chapter() -> Self {
        Self {
            light_dir: Vec3::Z,
            albedo: Vec3::splat(0.5),
            ambient: 0.1,
            intensity: 0.6,
            show_normals: 1,
        }
    }

    /// The shading formula, mirrored on the CPU for the checks below.
    pub fn shade(&self,
    normal: Vec3) -> Vec3 {
        let d = normal.normalize().dot(self.light_dir).max(0.0);
        self.albedo * (self.ambient + self.intensity * d)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dot_one_gives_035_and_dot_zero_gives_005() {
        let light = LightParams::chapter();
        // Facing the light dead-on: d = 1, color = 0.5*(0.1 + 0.6) = 0.35.
        let facing = light.shade(Vec3::Z);
        assert!(facing.abs_diff_eq(Vec3::splat(0.35), 1e-6));
        // Perpendicular: d = 0, only ambient, color = 0.5*0.1 = 0.05.
        let grazing = light.shade(Vec3::X);
        assert!(grazing.abs_diff_eq(Vec3::splat(0.05), 1e-6));
        // Turned away: dot < 0 is clamped to zero, not folded back.
        let back = light.shade(-Vec3::Z);
        assert!(back.abs_diff_eq(Vec3::splat(0.05), 1e-6));
    }

    #[test]
    fn averaged_perpendicular_normals_lose_length() {
        let mean = (Vec3::X + Vec3::Y) * 0.5;
        // The average of two unit normals is shorter than one unless they coincide.
        assert!((mean.length() - (0.5f32).sqrt()).abs() < 1e-6);
        // Renormalization restores unit length; the shader does this after interpolation.
        assert!((mean.normalize().length() - 1.0).abs() < 1e-6);
    }

    /// Uniform contract: vec3s at 16-byte alignment, scalars packed at 4, size 48.
    #[test]
    fn uniform_layout_matches_the_quoted_offsets() {
use encase::UniformBuffer;

        let mut buffer = UniformBuffer::new(Vec::<u8>::new());
        buffer
            .write(&LightParams::chapter())
            .expect("params fit the uniform contract");
        let bytes = buffer.into_inner();
        assert_eq!(bytes.len(), 48, "struct size rounds up to alignment 16");
        // light_dir at offset 0; albedo 0.5 at offset 16.
        let light_dir = Vec3::Z.to_array().map(f32::to_le_bytes);
        for (component, offset) in light_dir.iter().zip([0, 4, 8]) {
            assert_eq!(
                &bytes[offset..offset + 4],
                *component,
                "light_dir at {offset}"
            );
        }
        let half = 0.5_f32.to_le_bytes();
        for offset in [16, 20, 24] {
            assert_eq!(&bytes[offset..offset + 4], &half, "albedo at {offset}");
        }
        // Scalars pack at natural 4-byte alignment right after the vec3s.
        let ambient = 0.1_f32.to_le_bytes();
        let intensity = 0.6_f32.to_le_bytes();
        let one = 1_u32.to_le_bytes();
        assert_eq!(&bytes[28..32], &ambient, "ambient at 28");
        assert_eq!(&bytes[32..36], &intensity, "intensity at 32");
        assert_eq!(&bytes[36..40], &one, "show_normals at 36");
        // The trailing 8 bytes are the uniform size rounding, written as zeros.
        assert!(bytes[40..48].iter().all(|&b| b == 0));
    }
}
