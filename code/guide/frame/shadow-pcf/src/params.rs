use encase::ShaderType;
use glam::Vec3;

/// The B key cycle: zero, 5 mm, 50 mm of world depth over the 10 m light
/// range - the last detaches the shadow from its caster (Peter Panning).
pub const BIASES: [f32; 3] = [0.0, 0.0005, 0.005];

/// vec3s align to 16 bytes; the uniform occupies 64 bytes.
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
    /// Receiver bias added to the reference depth; the B key cycles it.
    pub bias: f32,
    /// 1 = the shadow lookup runs, 0 = plain Lambert; the H key flips it.
    pub shadows_on: u32,
    /// 1 = average a 3x3 block of comparisons (PCF); the P key flips it.
    pub pcf: u32,
}

impl SceneParams {
    pub fn chapter() -> Self {
        Self {
            light_dir: crate::scene::light_dir(),
            albedo: Vec3::splat(0.5),
            ambient: 0.1,
            intensity: 0.6,
            bias: 0.0,
            shadows_on: 1,
            pcf: 0,
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

    #[test]
    fn bias_values_in_world_units() {
        let range = crate::scene::FAR_L - crate::scene::NEAR_L;
        assert_eq!(range, 10.0);
        // 0.0005 -> 5 mm: covers a texel while the 12 mm gap stays shadowed.
        assert!(BIASES[1] * range < 0.012);
        // 0.005 -> 50 mm: exceeds the gap; the shadow detaches.
        assert!(BIASES[2] * range > 0.012);
    }
}
