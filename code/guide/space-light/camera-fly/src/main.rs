use std::error::Error;

use camera_fly::sample::CameraFly;
use framework::{Settings, run};
use tracing::Level;
use tracing_subscriber::fmt;

fn main() -> Result<(), Box<dyn Error>> {
    fmt()
        .with_max_level(Level::INFO)
        .init();
    run::<CameraFly>(Settings {
        title: "wgpu | Camera fly".into(),
        ..Settings::default()
    })
}
