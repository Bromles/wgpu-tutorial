use glam::Vec3;

/// Builds an orthonormal basis from `forward` and an `up` hint.
/// Returns (right, up, backward); rejects degenerate inputs.
pub fn orthonormal_basis(forward: Vec3,
up_hint: Vec3) -> Result<(Vec3, Vec3, Vec3), String> {
    if forward.length_squared() == 0.0 {
        return Err("forward direction must be nonzero".into());
    }
    let forward = forward.normalize();
    // Parallel, anti-parallel, or zero hints all give a zero cross.
    if up_hint.normalize_or_zero().cross(forward).length_squared() < 1e-10 {
        return Err("up hint is parallel to forward; basis is degenerate".into());
    }
    let right = forward.cross(up_hint).normalize();
    let up = right.cross(forward).normalize();
    // -forward keeps the triple right-handed like (X, Y, Z).
    Ok((right, up, -forward))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn length_and_normalization_follow_pythagoras() {
        let v = Vec3::new(3.0, 4.0, 0.0);
        assert_eq!(v.length(), 5.0);
        let n = v.normalize();
        assert!((n.x - 0.6).abs() < 1e-6 && (n.y - 0.8).abs() < 1e-6 && n.z.abs() < 1e-6);
        assert!((n.length() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn zero_vector_cannot_be_normalized() {
        assert_eq!(Vec3::ZERO.length(), 0.0);
        assert_eq!(Vec3::ZERO.normalize_or_zero(), Vec3::ZERO);
    }

    #[test]
    fn dot_measures_alignment() {
        // Perpendicular directions: dot = 0. Same direction: product of lengths.
        let x = Vec3::X;
        let y = Vec3::Y;
        assert!(x.dot(y).abs() < 1e-6);
        assert!((x.dot(2.0 * x) - 2.0).abs() < 1e-6);
        // The projection of a onto b is dot(a, b)/|b| for a unit b.
        let a = Vec3::new(1.0, 1.0, 0.0);
        assert!((a.dot(x) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cross_orientation_follows_the_right_hand_rule() {
        assert!((Vec3::X.cross(Vec3::Y) - Vec3::Z).length() < 1e-6);
        assert!((Vec3::Y.cross(Vec3::X) + Vec3::Z).length() < 1e-6);
        // Parallel arguments degenerate to zero.
        assert_eq!(Vec3::X.cross(Vec3::X), Vec3::ZERO);
    }

    #[test]
    fn basis_is_orthonormal_and_rejects_degenerate_input() {
        let (right, up, backward) = orthonormal_basis(Vec3::new(0.0, 0.0, -1.0), Vec3::Y).unwrap();
        for v in [right, up, backward] {
            assert!((v.length() - 1.0).abs() < 1e-5);
        }
        assert!(right.dot(up).abs() < 1e-5);
        assert!(up.dot(backward).abs() < 1e-5);
        // right x up = backward (the axis pointing against forward).
        assert!((right.cross(up) - backward).length() < 1e-5);
        assert!(orthonormal_basis(Vec3::ZERO, Vec3::Y).is_err());
        assert!(orthonormal_basis(Vec3::Y, Vec3::Y).is_err());
        // Anti-parallel and zero hints are degenerate too.
        assert!(orthonormal_basis(Vec3::Y, Vec3::new(0.0, -2.0, 0.0)).is_err());
        assert!(orthonormal_basis(Vec3::new(0.0, 0.0, -1.0), Vec3::ZERO).is_err());
    }

    #[test]
    fn local_coords_are_weights_of_world_axes() {
        // A point in a basis is the weighted sum of the basis vectors.
        let basis = (Vec3::X, Vec3::Y, Vec3::Z);
        let local = Vec3::new(2.0, -1.0, 0.5);
        let world = local.x * basis.0 + local.y * basis.1 + local.z * basis.2;
        assert!((world - local).length() < 1e-6);
    }
}
