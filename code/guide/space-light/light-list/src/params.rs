use encase::ShaderType;
use glam::{Mat4, Vec3, Vec4};

/// Lights the storage buffer holds; L cycles the active count, the buffer never resizes.
pub const MAX_LIGHTS: usize = 3;

/// Attenuation floor, metres, as in chapter 26a.
pub const R_MIN: f32 = 0.1;
/// Cone angles of chapter 26a; must match the shader constants.
pub const SPOT_COS_INNER: f32 = 0.9659258;
pub const SPOT_COS_OUTER: f32 = 0.8660254;

/// One light in two vec4s (pos+power, axis+spot flag); 32-byte stride, no padding.
#[derive(ShaderType, Debug, Clone, Copy, PartialEq)]
pub struct Light {
    pub pos: Vec4,
    pub spot: Vec4,
}

/// Two identical lights above the origin (their sum doubles the light) plus one offset; all plain points.
pub const LIGHTS: [Light; MAX_LIGHTS] = [
    Light {
        pos: Vec4::new(0.0, 0.0, 1.0, 0.4),
        spot: Vec4::new(0.0, 0.0, -1.0, 0.0),
    },
    Light {
        pos: Vec4::new(0.0, 0.0, 1.0, 0.4),
        spot: Vec4::new(0.0, 0.0, -1.0, 0.0),
    },
    Light {
        pos: Vec4::new(1.4, 0.0, 1.0, 0.4),
        spot: Vec4::new(0.0, 0.0, -1.0, 0.0),
    },
];

/// Camera chain plus the count as f32 (no manual padding; shader converts back); 80 bytes.
#[derive(ShaderType, Debug, Clone, Copy, PartialEq)]
pub struct Params {
    pub view_proj: Mat4,
    pub count: f32,
}

/// Direct contribution of one light at one point: the shader formula mirrored on the CPU.
pub fn contribution(light: Light, p: Vec3, n: Vec3) -> f32 {
    let to_light = light.pos.truncate() - p;
    let r = to_light.length();
    let l = to_light / r;
    let attenuation = 1.0 / (r * r).max(R_MIN * R_MIN);
    let mut c = light.pos.w * attenuation * n.dot(l).max(0.0);
    if light.spot.w == 1.0 {
        let d = (-l).dot(light.spot.truncate());
        let t = ((d - SPOT_COS_OUTER) / (SPOT_COS_INNER - SPOT_COS_OUTER)).clamp(0.0, 1.0);
        c *= t;
    }
    c
}

/// Sum of the first `count` contributions plus ambient: the shader's per-pixel model.
pub fn shade(p: Vec3, n: Vec3, count: u32, ambient: f32) -> f32 {
    let mut sum = 0.0;
    for light in &LIGHTS[..(count as usize).min(MAX_LIGHTS)] {
        sum += contribution(*light, p, n);
    }
    0.5 * (ambient + sum)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The cone cosines appear in the shader too; a silent typo would shift both cone edges.
    #[test]
    fn spot_constants_match_their_angles() {
        assert!((SPOT_COS_INNER - 15.0f32.to_radians().cos()).abs() < 1e-6);
        assert!((SPOT_COS_OUTER - 30.0f32.to_radians().cos()).abs() < 1e-6);
        const { assert!(SPOT_COS_INNER > SPOT_COS_OUTER) };
    }

    /// Two identical sources double the linear light; albedo 0.5 scales the 0.4 power to 0.2 each.
    #[test]
    fn two_identical_lights_double_the_linear_light() {
        let one = shade(Vec3::ZERO, Vec3::Z, 1, 0.0);
        let two = shade(Vec3::ZERO, Vec3::Z, 2, 0.0);
        assert!((one - 0.2).abs() < 1e-6);
        assert!((two - 0.4).abs() < 1e-6);
        assert!((two - 2.0 * one).abs() < 1e-6);
    }

    /// Ambient belongs to the scene, not to any source.
    #[test]
    fn zero_lights_leave_ambient_only() {
        assert!((shade(Vec3::ZERO, Vec3::Z, 0, 0.1) - 0.05).abs() < 1e-6);
    }
}
