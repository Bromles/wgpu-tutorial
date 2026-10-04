use std::error::Error;

use full_frame::sample::FullFrame;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    framework::run::<FullFrame>(framework::Settings {
        title: "wgpu | Full frame".into(),
        device_descriptor: wgpu::DeviceDescriptor {
            label: Some("Full frame device"),
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
