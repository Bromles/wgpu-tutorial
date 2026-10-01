use encase::ShaderType;
use glam::{Mat3, Mat4, Vec3};

/// Model moves positions; the normal matrix moves directions that must stay perpendicular.
#[derive(ShaderType, Debug, Clone, Copy, PartialEq)]
pub struct ModelParams {
    pub model: Mat4,
    pub normal_matrix: Mat4,
}

impl ModelParams {
    /// From an axis scale; a zero component is rejected (no inverse, no normal).
    pub fn from_scale(scale: Vec3) -> Result<Self, String> {
        if scale.cmpeq(Vec3::ZERO).any() {
            return Err(format!(
                "scale {scale} has a zero component; the normal matrix is undefined"
            ));
        }
        let model = Mat4::from_scale(scale);
        // Linear 3x3 part only: translation never applies to a direction.
        let normal_matrix = Mat4::from_mat3(Mat3::from_mat4(model).inverse().transpose());
        Ok(Self {
            model,
            normal_matrix,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Under X times 2 the naive M*n stops being perpendicular to the face.
    #[test]
    fn naive_model_normal_breaks_perpendicularity() {
        let t = Vec3::new(1.0, 1.0, 0.0);
        let n = Vec3::new(1.0, -1.0, 0.0);
        let model = Mat4::from_scale(Vec3::new(2.0, 1.0, 1.0));
        let mt = model.transform_vector3(t);
        let mn = model.transform_vector3(n);
        assert!((mt - Vec3::new(2.0, 1.0, 0.0)).length() < 1e-6);
        assert!((mn - Vec3::new(2.0, -1.0, 0.0)).length() < 1e-6);
        // n*t = 0 before the transform, 3 after: the naive normal is wrong.
        assert!((mt.dot(mn) - 3.0).abs() < 1e-6);
    }

    #[test]
    fn inverse_transpose_keeps_perpendicularity() {
        let t = Vec3::new(1.0, 1.0, 0.0);
        let n = Vec3::new(1.0, -1.0, 0.0);
        let params = ModelParams::from_scale(Vec3::new(2.0, 1.0, 1.0)).unwrap();
        let mt = params.model.transform_vector3(t);
        let nn = params.normal_matrix.transform_vector3(n);
        assert!((nn - Vec3::new(0.5, -1.0, 0.0)).length() < 1e-6);
        // The defining property: (N*n)*(M*t) = n*t = 0.
        assert!(mt.dot(nn).abs() < 1e-6);
        // Normalizing the wrong normal does not restore it either.
        let wrong = params.model.transform_vector3(n).normalize();
        assert!((mt.dot(wrong) - 3.0 / 5.0_f32.sqrt()).abs() < 1e-6);
    }

    #[test]
    fn zero_scale_is_rejected_not_masked() {
        assert!(ModelParams::from_scale(Vec3::new(0.0, 1.0, 1.0)).is_err());
        assert!(ModelParams::from_scale(Vec3::new(1.0, 1.0, 1.0)).is_ok());
    }
}
