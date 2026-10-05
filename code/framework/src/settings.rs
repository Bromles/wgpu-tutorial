use wgpu::{DeviceDescriptor, Features, Limits};

/// Example-supplied configuration, including device requirements.
pub struct Settings {
    pub title: String,
    pub inner_size: (u32, u32),
    pub device_descriptor: DeviceDescriptor<'static>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            title: "wgpu".to_string(),
            inner_size: (800, 600),
            device_descriptor: DeviceDescriptor {
                label: Some("Framework device"),
                required_features: Features::empty(),
                required_limits: Limits::default(),
                ..Default::default()
            },
        }
    }
}
