use encase::ShaderType;
use glam::Vec3;

/// vec3 fields align to 16 bytes; the uniform occupies 48 bytes.
#[derive(ShaderType, Debug, Clone, Copy)]
pub struct SceneParams {
    /// Unit direction from the surface toward the light source.
    pub light_dir: Vec3,
    /// Base surface color (fraction of light the material reflects).
    pub albedo: Vec3,
    /// Constant term: brightness that does not depend on visibility.
    pub ambient: f32,
    /// Strength of the direct (Lambert) term.
    pub intensity: f32,
    /// 1 = the shadow lookup runs, 0 = plain Lambert; the H key flips it.
    pub shadows_on: u32,
}

impl SceneParams {
    pub fn chapter() -> Self {
        Self {
            light_dir: crate::scene::light_dir(),
            albedo: Vec3::splat(0.5),
            ambient: 0.1,
            intensity: 0.6,
            shadows_on: 1,
        }
    }

    /// Mirrors the shader formula for the verification checks.
    pub fn shade(&self, normal: Vec3, visibility: f32) -> Vec3 {
        let d = normal.normalize().dot(self.light_dir).max(0.0);
        self.albedo * (self.ambient + self.intensity * d * visibility)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floor_lambert_and_shadow_floor_control_numbers() {
        let params = SceneParams::chapter();
        // Floor normal +Y: d = L.y ≈ 0.8165; lit ≈ 0.29495 (code 148).
        let lit = params.shade(Vec3::Y, 1.0);
        assert!(lit.abs_diff_eq(Vec3::splat(0.29495), 1e-4));
        // Fully blocked: only ambient survives, 0.5*0.1 = 0.05 (code 63).
        let blocked = params.shade(Vec3::Y, 0.0);
        assert!(blocked.abs_diff_eq(Vec3::splat(0.05), 1e-6));
        // Acceptance bound: shadowed probe at or below ambient + 0.15 * intensity * d.
        let bound = 0.5 * (0.1 + 0.15 * 0.6 * params.light_dir.y);
        assert!(blocked.x <= bound);
        assert!(lit.x > bound);
    }
}
