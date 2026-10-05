use std::error::Error;

use framework::{Settings, run};
use wgpu::{DeviceDescriptor, Features, Limits};
use tracing::Level;
use tracing_subscriber::fmt;
use draw_immediates::sample::DrawImmediates;

fn main() -> Result<(), Box<dyn Error>> {
    fmt()
        .with_max_level(Level::INFO)
        .init();
    run::<DrawImmediates>(Settings {
        title: "wgpu | Draw immediates".into(),
        device_descriptor: DeviceDescriptor {
            label: Some("Immediates device"),
            required_features: Features::IMMEDIATES,
            required_limits: Limits {
                max_immediate_size: 4,
                ..Limits::default()
            },
            ..Default::default()
        },
        ..Settings::default()
    })
}
