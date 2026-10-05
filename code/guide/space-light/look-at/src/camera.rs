use glam::Vec3;


use glam::Mat4;

/// Explicit camera transform: returns (right, up, backward, position), the camera matrix columns.
pub fn camera_axes(
    eye: Vec3,
    target: Vec3,
    up_hint: Vec3,
) -> Result<(Vec3, Vec3, Vec3, Vec3), String> {
    let forward = target - eye;
    if forward.length_squared() == 0.0 {
        return Err("eye and target coincide; view direction is undefined".into());
    }
    let forward = forward.normalize();
    if up_hint.normalize_or_zero().cross(forward).length_squared() < 1e-10 {
        return Err("up hint is parallel to the view direction".into());
    }
    let right = forward.cross(up_hint).normalize();
    let up = right.cross(forward).normalize();
    Ok((right, up, -forward, eye))
}

/// Hand-built view matrix: the world in the camera basis, minus the eye; equals inverse(camera).
pub fn manual_view(eye: Vec3, target: Vec3, up_hint: Vec3) -> Result<Mat4, String> {
    let (right, up, backward, eye) = camera_axes(eye, target, up_hint)?;
    Ok(Mat4::from_cols(
        right.extend(0.0),
        up.extend(0.0),
        backward.extend(0.0),
        eye.extend(1.0),
    )
    .inverse())
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::camera::rh::view::look_at_mat4;

    #[test]
    fn world_origin_maps_to_negative_five_on_z() {
        let view = manual_view(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, Vec3::Y).unwrap();
        let origin = view.transform_point3(Vec3::ZERO);
        assert!(origin.abs_diff_eq(Vec3::new(0.0, 0.0, -5.0), 1e-5));
    }

    #[test]
    fn moving_camera_right_moves_the_world_left() {
        // A camera shifted +1 along X (no rotation) sees the origin at x = -1.
        let view =
            manual_view(Vec3::new(1.0, 0.0, 5.0), Vec3::new(1.0, 0.0, 0.0), Vec3::Y).unwrap();
        let origin = view.transform_point3(Vec3::ZERO);
        assert!(origin.abs_diff_eq(Vec3::new(-1.0, 0.0, -5.0), 1e-5));
    }

    #[test]
    fn manual_view_matches_glam_look_at_rh() {
        let eye = Vec3::new(0.4, 0.7, 5.0);
        let target = Vec3::new(-0.2, 0.1, 0.0);
        let manual = manual_view(eye, target, Vec3::Y).unwrap();
        let library = look_at_mat4(eye, target, Vec3::Y);
        for (m, l) in manual.to_cols_array().iter().zip(library.to_cols_array()) {
            assert!((m - l).abs() < 1e-5, "manual {m} vs look_at_rh {l}");
        }
    }

    #[test]
    fn camera_times_view_is_identity() {
        let eye = Vec3::new(0.3, -0.2, 4.0);
        let target = Vec3::new(0.1, 0.2, 0.0);
        let (right, up, backward, eye) = camera_axes(eye, target, Vec3::Y).unwrap();
        let camera = Mat4::from_cols(
            right.extend(0.0),
            up.extend(0.0),
            backward.extend(0.0),
            eye.extend(1.0),
        );
        let view = manual_view(eye, target, Vec3::Y).unwrap();
        let product = camera * view;
        for (index, value) in product.to_cols_array().iter().enumerate() {
            let expected = Mat4::IDENTITY.to_cols_array()[index];
            assert!((value - expected).abs() < 1e-5);
        }
    }

    #[test]
    fn degenerate_inputs_are_rejected() {
        assert!(manual_view(Vec3::ZERO, Vec3::ZERO, Vec3::Y).is_err());
        assert!(manual_view(Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.0, 5.0, 0.0), Vec3::Y).is_err());
    }
}
