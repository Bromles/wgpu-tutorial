use std::f32::consts::FRAC_PI_2;

use glam::{Mat4, Vec3};
use winit::keyboard::KeyCode;

use crate::Input;

pub struct Camera {
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub speed: f32,
    pub sensitivity: f32,
}

impl Camera {
    pub fn new(position: Vec3, yaw: f32, pitch: f32) -> Self {
        Self {
            position,
            yaw,
            pitch,
            speed: 5.0,
            sensitivity: 0.003,
        }
    }

    pub fn direction(&self) -> Vec3 {
        Vec3::new(
            -self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            -self.yaw.cos() * self.pitch.cos(),
        )
    }

    pub fn forward(&self) -> Vec3 {
        Vec3::new(-self.yaw.sin(), 0.0, -self.yaw.cos())
    }

    pub fn right(&self) -> Vec3 {
        Vec3::new(self.yaw.cos(), 0.0, -self.yaw.sin())
    }

    pub fn up(&self) -> Vec3 {
        Vec3::Y
    }

    pub fn view_matrix(&self) -> Mat4 {
        glam::camera::rh::view::look_to_mat4(self.position, self.direction(), Vec3::Y)
    }

    pub fn update(&mut self, dt: f32, input: &Input) {
        if input.mouse_button_pressed(1) {
            let (dx, dy) = input.mouse_delta();
            self.yaw -= dx as f32 * self.sensitivity;
            self.pitch -= dy as f32 * self.sensitivity;
            self.pitch = self.pitch.clamp(-FRAC_PI_2 + 0.01, FRAC_PI_2 - 0.01);
        }

        let forward = self.forward();
        let right = self.right();
        let mut velocity = Vec3::ZERO;

        if input.key_pressed(KeyCode::KeyW) {
            velocity += forward;
        }
        if input.key_pressed(KeyCode::KeyS) {
            velocity -= forward;
        }
        if input.key_pressed(KeyCode::KeyD) {
            velocity += right;
        }
        if input.key_pressed(KeyCode::KeyA) {
            velocity -= right;
        }
        if input.key_pressed(KeyCode::Space) {
            velocity.y += 1.0;
        }
        if input.key_pressed(KeyCode::ShiftLeft) {
            velocity.y -= 1.0;
        }

        if velocity.length_squared() > 0.0 {
            self.position += velocity.normalize() * self.speed * dt;
        }
    }
}
