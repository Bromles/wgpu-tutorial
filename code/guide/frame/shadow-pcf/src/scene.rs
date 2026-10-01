use glam::{Mat4, Vec3};

/// Enough +Y to come from above; the equal +X/+Z lean keeps checks simple.
const LIGHT_RAW: Vec3 = Vec3::new(0.5, 1.0, 0.5);

/// L of the Lambert term and the light camera's view axis.
pub fn light_dir() -> Vec3 {
    LIGHT_RAW.normalize()
}

/// Fixed viewer camera of the chapter: above and in front of the scene.
pub const EYE: Vec3 = Vec3::new(2.5, 2.0, 4.0);
pub const TARGET: Vec3 = Vec3::new(0.0, 0.3, 0.0);
pub const FOV_Y: f32 = std::f32::consts::FRAC_PI_3;
/// Fixed 4:3 aspect keeps the projection independent of the window.
pub const ASPECT: f32 = 4.0 / 3.0;
pub const NEAR: f32 = 0.1;
pub const FAR: f32 = 50.0;

/// Axis-aligned bounds of the cube caster: a 1x1 m square lifted 1 cm
/// above the floor, i.e. the center sits at (0, 0.51, 0).
pub const CUBE_MIN: Vec3 = Vec3::new(-0.5, 0.01, -0.5);
pub const CUBE_MAX: Vec3 = Vec3::new(0.5, 1.01, 0.5);

/// The light looks from 10 units along L; the box is 6x6 m.
pub const LIGHT_TARGET: Vec3 = Vec3::new(0.0, 0.5, 0.0);
const LIGHT_DISTANCE: f32 = 10.0;
const LIGHT_HALF_WIDTH: f32 = 3.0;
/// The 10 m span is the unit behind the bias values: 0.0005 = 5 mm.
pub const NEAR_L: f32 = 5.0;
pub const FAR_L: f32 = 15.0;

/// Far enough along L to fit the scene between NEAR_L and FAR_L.
pub fn light_eye() -> Vec3 {
    LIGHT_TARGET + light_dir() * LIGHT_DISTANCE
}

pub fn camera_view_proj() -> Mat4 {
    glam::camera::rh::proj::directx::perspective(FOV_Y, ASPECT, NEAR, FAR)
        * glam::camera::rh::view::look_at_mat4(EYE, TARGET, Vec3::Y)
}

/// Orthographic box for parallel rays; applied as P·V from the right.
pub fn light_view_proj() -> Mat4 {
    glam::camera::rh::proj::directx::orthographic(
        -LIGHT_HALF_WIDTH,
        LIGHT_HALF_WIDTH,
        -LIGHT_HALF_WIDTH,
        LIGHT_HALF_WIDTH,
        NEAR_L,
        FAR_L,
    ) * glam::camera::rh::view::look_at_mat4(light_eye(), LIGHT_TARGET, Vec3::Y)
}

/// Pure translation along X, so the attribute is the world normal.
pub fn cube_model(offset_x: f32) -> Mat4 {
    Mat4::from_translation(Vec3::new(offset_x, 0.0, 0.0))
}

/// Where the cube's axis casts: `C - L * (C.y / L.y)`; the analytic control point.
pub fn cube_shadow_center(cube_offset_x: f32) -> Vec3 {
    let l = light_dir();
    let center = Vec3::new(cube_offset_x, 0.51, 0.0);
    center - l * (center.y / l.y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_dir_is_unit_and_leans_equally_in_x_and_z() {
        let l = light_dir();
        assert!((l.length() - 1.0).abs() < 1e-6);
        // (0.5, 1, 0.5)/sqrt(1.5): 0.51 m casts 0.255 m sideways.
        assert!((l.x - 0.5 / 1.5_f32.sqrt()).abs() < 1e-6);
        assert!((l.x - l.z).abs() < 1e-6);
        assert!(l.y > l.x);
    }

    /// Margins keep everything out of the outside-frustum fallback (no dark border).
    #[test]
    fn light_frustum_covers_floor_and_all_cube_poses() {
        let light = light_view_proj();
        let mut corners = Vec::new();
        for x in [-2.0, 2.0] {
            for z in [-2.0, 2.0] {
                corners.push(Vec3::new(x, 0.0, z));
            }
        }
        for offset in [0.0, 1.0, -1.0] {
            for x in [offset - 0.5, offset + 0.5] {
                for y in [0.01, 1.01] {
                    for z in [-0.5, 0.5] {
                        corners.push(Vec3::new(x, y, z));
                    }
                }
            }
        }
        for corner in corners {
            let clip = light * corner.extend(1.0);
            let ndc = Vec3::new(clip.x / clip.w, clip.y / clip.w, clip.z / clip.w);
            assert!(
                ndc.x > -0.99 && ndc.x < 0.99 && ndc.y > -0.99 && ndc.y < 0.99,
                "{corner:?} lands at ndc {ndc:?}"
            );
            assert!(ndc.z > 0.0 && ndc.z < 1.0, "{corner:?} depth {ndc:?}");
        }
    }

    #[test]
    fn cube_shadow_center_matches_the_analytic_displacement() {
        // Lean 0.5: 0.51 m of height casts 0.255 m in -X and -Z.
        assert!(cube_shadow_center(0.0).abs_diff_eq(Vec3::new(-0.255, 0.0, -0.255), 1e-3));
        assert!(cube_shadow_center(1.0).abs_diff_eq(Vec3::new(0.745, 0.0, -0.255), 1e-3));
    }
}
