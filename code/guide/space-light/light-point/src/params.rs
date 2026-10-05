use encase::ShaderType;
use glam::{Mat4, Vec3};

///  Attenuation floor: the denominator never drops below R_MIN^2, staying finite up close.
pub const R_MIN: f32 = 0.1;

/// One positional source: a point light until the cone is switched on.
/// vec3s align to 16 with the next scalar in the slot's tail; 112 bytes.
#[derive(ShaderType, Debug, Clone, Copy, PartialEq)]
pub struct Params {
    pub view_proj: Mat4,
    /// World-space position of the source, metres.
    pub light_pos: Vec3,
    /// Radiated power in linear units; not lumens.
    pub power: f32,
    /// Unit cone axis (used when `spot_on` is 1).
    pub spot_dir: Vec3,
    /// Cosine of the fully lit cone angle (15 degrees).
    pub spot_cos_inner: f32,
    /// Cosine of the dark cone angle (30 degrees).
    pub spot_cos_outer: f32,
    /// 1 = mask the light with the cone, 0 = plain point.
    pub spot_on: u32,
}

impl Params {
    /// Chapter start: point source 2 m above the origin, axis pointing down the plane.
    pub fn new(view_proj: Mat4) -> Self {
        Self {
            view_proj,
            light_pos: Vec3::new(0.0, 0.0, 2.0),
            power: 0.4,
            spot_dir: Vec3::new(0.0, 0.0, -1.0),
            spot_cos_inner: 15.0_f32.to_radians().cos(),
            spot_cos_outer: 30.0_f32.to_radians().cos(),
            spot_on: 0,
        }
    }

    /// Direct contribution at one surface point: the shader formula mirrored on the CPU.
    pub fn contribution(&self,
    p: Vec3,
    n: Vec3) -> f32 {
        // L points from the surface toward the source; r is that distance.
        let to_light = self.light_pos - p;
        let r = to_light.length();
        // The source sits exactly on the surface point: no direction is
        // defined, so there is nothing to contribute (and no safe divide).
        if r == 0.0 {
            return 0.0;
        }
        let l = to_light / r;
        let attenuation = 1.0 / (r * r).max(R_MIN * R_MIN);
        let mut contribution = self.power * attenuation * n.dot(l).max(0.0);
        if self.spot_on == 1 {
            // Cosine between the cone axis and -L; the fade is linear in the cosines.
            let d = (-l).dot(self.spot_dir);
            let t = ((d - self.spot_cos_outer) / (self.spot_cos_inner - self.spot_cos_outer))
                .clamp(0.0, 1.0);
            contribution *= t;
        }
        contribution
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// N*L = 1 with power 0.4: diffuse 0.2 at r = 1 m, 0.05 at r = 2 m.
    #[test]
    fn one_and_two_metres_give_02_and_005() {
        let mut params = Params::new(Mat4::IDENTITY);
        params.light_pos = Vec3::new(0.0, 0.0, 1.0);
        let diffuse = 0.5 * params.contribution(Vec3::ZERO, Vec3::Z);
        assert!((diffuse - 0.2).abs() < 1e-6);
        params.light_pos = Vec3::new(0.0, 0.0, 2.0);
        let diffuse = 0.5 * params.contribution(Vec3::ZERO, Vec3::Z);
        assert!((diffuse - 0.05).abs() < 1e-6);
    }

    /// Full contribution on the cone axis, zero at the outer cosine, linear between.
    #[test]
    fn spot_fades_linearly_between_the_cone_cosines() {
        let mut params = Params::new(Mat4::IDENTITY);
        params.light_pos = Vec3::new(0.0, 0.0, 1.0);
        params.spot_on = 1;
        // On the axis: d = 1, above the inner cosine, t = 1.
        let axis = params.contribution(Vec3::ZERO, Vec3::Z);
        assert!((axis - 0.4).abs() < 1e-6);
        // At the outer cosine the ray leaves the plane at tan(30 deg): t = 0.
        let edge = params.contribution(Vec3::new(0.57735027, 0.0, 0.0), Vec3::Z);
        assert!(edge.abs() < 1e-6);
        // Halfway between the cosines (by value, not by angle): t = 0.5.
        let mid_cos = (params.spot_cos_inner + params.spot_cos_outer) * 0.5;
        let x = (1.0 / (mid_cos * mid_cos) - 1.0).sqrt();
        let p = Vec3::new(x, 0.0, 0.0);
        params.spot_on = 1;
        let with_cone = params.contribution(p, Vec3::Z);
        params.spot_on = 0;
        let without_cone = params.contribution(p, Vec3::Z);
        assert!((with_cone / without_cone - 0.5).abs() < 1e-5);
    }
}
