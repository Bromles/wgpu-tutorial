use std::collections::HashSet;

use winit::keyboard::{KeyCode, PhysicalKey};

#[derive(Default)]
pub struct Input {
    pressed_keys: HashSet<KeyCode>,
    mouse_delta: (f64, f64),
    mouse_buttons: HashSet<u64>,
}

impl Input {
    pub fn key_pressed(&self, key: KeyCode) -> bool {
        self.pressed_keys.contains(&key)
    }

    pub fn mouse_delta(&self) -> (f64, f64) {
        self.mouse_delta
    }

    pub fn mouse_button_pressed(&self, button: u64) -> bool {
        self.mouse_buttons.contains(&button)
    }

    pub fn press_key(&mut self, key: KeyCode) {
        self.pressed_keys.insert(key);
    }

    pub fn release_key(&mut self, key: KeyCode) {
        self.pressed_keys.remove(&key);
    }

    pub fn set_mouse_delta(&mut self, dx: f64, dy: f64) {
        self.mouse_delta = (dx, dy);
    }

    pub fn press_mouse(&mut self, button: u64) {
        self.mouse_buttons.insert(button);
    }

    pub fn release_mouse(&mut self, button: u64) {
        self.mouse_buttons.remove(&button);
    }

    pub fn clear_delta(&mut self) {
        self.mouse_delta = (0.0, 0.0);
    }
}

fn key_code(physical_key: PhysicalKey) -> Option<KeyCode> {
    match physical_key {
        PhysicalKey::Code(code) => Some(code),
        _ => None,
    }
}

pub fn extract_key(physical_key: PhysicalKey) -> Option<KeyCode> {
    key_code(physical_key)
}
