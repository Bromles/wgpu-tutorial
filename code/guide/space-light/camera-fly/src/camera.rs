use std::collections::HashSet;

use glam::{Mat4, Vec3};
use winit::keyboard::KeyCode;

/// Flight speed, meters per second.
pub const SPEED: f32 = 2.0;
/// Radians of rotation per unit of mouse delta; not scaled by `dt` (a delta already covers a frame).
pub const SENSITIVITY: f32 = 0.0025;
/// Stops just short of vertical, where `forward` becomes parallel to the world up.
pub const PITCH_LIMIT: f32 = std::f32::consts::FRAC_PI_2 - 0.01;

/// The pose the `R` key returns to.
pub const START_POSITION: Vec3 = Vec3::new(0.0, 1.5, 4.0);
pub const START_YAW: f32 = 0.0;
pub const START_PITCH: f32 = -0.3;

/// Pose: position plus two angles; one `update` per frame applies accumulated input.
pub struct Camera {
    /// Where the eye is, meters.
    pub position: Vec3,
    /// Horizontal angle: 0 looks along -Z, positive turns to the right (+X).
    pub yaw: f32,
    /// Vertical angle: positive looks up, clamped to +-PITCH_LIMIT.
    pub pitch: f32,
    /// Mouse delta accumulated since the last update.
    pending_delta: (f32, f32),
}

impl Camera {
    pub fn new(position: Vec3, yaw: f32, pitch: f32) -> Self {
        Self {
            position,
            yaw,
            pitch,
            pending_delta: (0.0, 0.0),
        }
    }

    /// Unit view direction: start at `(0, 0, -1)`, tilt by pitch around X, then turn by yaw.
    /// `forward = (sin(yaw)·cos(pitch), sin(pitch), -cos(yaw)·cos(pitch))`.
    pub fn forward(&self) -> Vec3 {
        Vec3::new(
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            -self.yaw.cos() * self.pitch.cos(),
        )
    }

    /// Right of the view, horizontal at any pitch: `forward x world_up` (chapter 17).
    pub fn right(&self) -> Vec3 {
        self.forward().cross(Vec3::Y).normalize()
    }

    /// View matrix: the world in the camera basis (chapter 18).
    pub fn view_matrix(&self) -> Mat4 {
        glam::camera::rh::view::look_at_mat4(self.position, self.position + self.forward(), Vec3::Y)
    }

    /// Accumulates raw mouse delta; consumed by the next `update`.
    pub fn add_mouse_delta(&mut self, dx: f32, dy: f32) {
        self.pending_delta.0 += dx;
        self.pending_delta.1 += dy;
    }

    /// One step per frame: mouse delta without `dt`, movement with `dt` at `SPEED`.
    /// The pressed-direction sum is normalized so diagonals are not faster.
    pub fn update(&mut self, dt: f32, keys: &HashSet<KeyCode>) {
        self.yaw += self.pending_delta.0 * SENSITIVITY;
        self.pitch =
            (self.pitch - self.pending_delta.1 * SENSITIVITY).clamp(-PITCH_LIMIT, PITCH_LIMIT);
        self.pending_delta = (0.0, 0.0);

        let forward = self.forward();
        let right = self.right();
        let mut direction = Vec3::ZERO;
        if keys.contains(&KeyCode::KeyW) {
            direction += forward;
        }
        if keys.contains(&KeyCode::KeyS) {
            direction -= forward;
        }
        if keys.contains(&KeyCode::KeyD) {
            direction += right;
        }
        if keys.contains(&KeyCode::KeyA) {
            direction -= right;
        }
        if keys.contains(&KeyCode::Space) {
            direction += Vec3::Y;
        }
        if keys.contains(&KeyCode::ShiftLeft) || keys.contains(&KeyCode::ShiftRight) {
            direction -= Vec3::Y;
        }
        if direction.length_squared() > 0.0 {
            self.position += direction.normalize() * SPEED * dt;
        }
    }

    /// Returns the pose the `R` key restores.
    pub fn reset(&mut self) {
        self.position = START_POSITION;
        self.yaw = START_YAW;
        self.pitch = START_PITCH;
        self.pending_delta = (0.0, 0.0);
    }

    /// Drops deltas collected while unfocused so they don't fire on refocus.
    pub fn clear_pending_delta(&mut self) {
        self.pending_delta = (0.0, 0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_keys() -> HashSet<KeyCode> {
        HashSet::new()
    }

    fn keys(codes: &[KeyCode]) -> HashSet<KeyCode> {
        codes.iter().copied().collect()
    }

    #[test]
    fn one_second_of_flight_covers_two_meters_at_any_frame_rate() {
        let forward_key = keys(&[KeyCode::KeyW]);
        let mut slow = Camera::new(Vec3::ZERO, 0.0, 0.0);
        for _ in 0..30 {
            slow.update(1.0 / 30.0, &forward_key);
        }
        let mut fast = Camera::new(Vec3::ZERO, 0.0, 0.0);
        for _ in 0..120 {
            fast.update(1.0 / 120.0, &forward_key);
        }
        // 2 m/s for one second: distance 2 m along -Z regardless of the rate.
        assert!((slow.position.length() - 2.0).abs() < 1e-4);
        assert!((fast.position.length() - 2.0).abs() < 1e-4);
        assert!((slow.position - fast.position).length() < 1e-4);
    }

    #[test]
    fn motion_stops_without_keys() {
        let mut camera = Camera::new(Vec3::ZERO, 0.0, 0.0);
        camera.update(1.0, &no_keys());
        assert_eq!(camera.position, Vec3::ZERO);
    }

    #[test]
    fn mouse_deltas_accumulate_before_update() {
        let mut camera = Camera::new(Vec3::ZERO, 0.0, 0.0);
        camera.add_mouse_delta(3.0, 0.0);
        camera.add_mouse_delta(4.0, 0.0);
        camera.update(0.0, &no_keys());
        // 3 + 4 units of delta give 7 units worth of turning.
        assert!((camera.yaw - 7.0 * SENSITIVITY).abs() < 1e-6);
        // The accumulator is consumed: a second update changes nothing.
        camera.update(0.0, &no_keys());
        assert!((camera.yaw - 7.0 * SENSITIVITY).abs() < 1e-6);
    }

    #[test]
    fn yaw_zero_looks_along_negative_z_and_positive_yaw_turns_right() {
        let straight = Camera::new(Vec3::ZERO, 0.0, 0.0);
        assert!(
            straight
                .forward()
                .abs_diff_eq(Vec3::new(0.0, 0.0, -1.0), 1e-6)
        );
        let turned = Camera::new(Vec3::ZERO, std::f32::consts::FRAC_PI_2, 0.0);
        assert!(turned.forward().abs_diff_eq(Vec3::new(1.0, 0.0, 0.0), 1e-6));
    }

    #[test]
    fn pitch_clamp_keeps_the_view_basis_buildable() {
        let mut camera = Camera::new(Vec3::ZERO, 0.0, 0.0);
        camera.add_mouse_delta(0.0, -1.0e6);
        camera.update(0.0, &no_keys());
        assert!((camera.pitch - PITCH_LIMIT).abs() < 1e-6);
        // forward stays non-parallel to world up: the basis cross keeps nonzero length.
        assert!(camera.forward().cross(Vec3::Y).length() > 0.009);
    }

    #[test]
    fn diagonal_motion_is_normalized() {
        let diagonal = keys(&[KeyCode::KeyW, KeyCode::KeyD]);
        let mut camera = Camera::new(Vec3::ZERO, 0.0, 0.0);
        camera.update(0.5, &diagonal);
        // Half a second at 2 m/s: exactly 1 m, not sqrt(2) times more.
        assert!((camera.position.length() - 1.0).abs() < 1e-5);
        let expected = Vec3::new(
            std::f32::consts::FRAC_1_SQRT_2,
            0.0,
            -std::f32::consts::FRAC_1_SQRT_2,
        );
        assert!(camera.position.normalize().abs_diff_eq(expected, 1e-5));
    }

    #[test]
    fn view_places_the_world_origin_in_front_of_the_camera() {
        let camera = Camera::new(Vec3::new(0.0, 0.0, 5.0), 0.0, 0.0);
        let origin = camera.view_matrix().transform_point3(Vec3::ZERO);
        assert!(origin.abs_diff_eq(Vec3::new(0.0, 0.0, -5.0), 1e-5));
    }

    #[test]
    fn reset_returns_the_start_pose() {
        let mut camera = Camera::new(Vec3::ZERO, 1.0, 0.5);
        camera.add_mouse_delta(10.0, 10.0);
        camera.reset();
        assert_eq!(camera.position, START_POSITION);
        assert_eq!(camera.yaw, START_YAW);
        assert_eq!(camera.pitch, START_PITCH);
    }
}
