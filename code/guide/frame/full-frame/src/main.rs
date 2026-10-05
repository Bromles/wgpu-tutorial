use std::error::Error;

use full_frame::sample::FullFrame;
use framework::{Settings, run};
use tracing::Level;
use tracing_subscriber::fmt;
use wgpu::{DeviceDescriptor, Features, Limits};

fn main() -> Result<(), Box<dyn Error>> {
    fmt()
        .with_max_level(Level::INFO)
        .init();
    run::<FullFrame>(Settings {
        title: "wgpu | Full frame".into(),
        device_descriptor: DeviceDescriptor {
            label: Some("Full frame device"),
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
