use std::error::Error;

use framework::{Settings, run};
use scene_objects::sample::SceneObjects;
use tracing::Level;
use tracing_subscriber::fmt;
use wgpu::{DeviceDescriptor, Features, Limits};

fn main() -> Result<(), Box<dyn Error>> {
    fmt().with_max_level(Level::INFO).init();
    run::<SceneObjects>(Settings {
        title: "wgpu | Scene objects".into(),
        device_descriptor: DeviceDescriptor {
            label: Some("Scene objects device"),
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
