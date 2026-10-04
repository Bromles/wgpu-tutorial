use std::error::Error;

use scene_objects::sample::SceneObjects;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    framework::run::<SceneObjects>(framework::Settings {
        title: "wgpu | Scene objects".into(),
        device_descriptor: wgpu::DeviceDescriptor {
            label: Some("Scene objects device"),
            required_features: wgpu::Features::IMMEDIATES,
            required_limits: wgpu::Limits {
                max_immediate_size: 4,
                ..wgpu::Limits::default()
            },
            ..Default::default()
        },
        ..framework::Settings::default()
    })
}
