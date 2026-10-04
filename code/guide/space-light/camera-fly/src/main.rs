use std::error::Error;

use camera_fly::sample::CameraFly;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    framework::run::<CameraFly>(framework::Settings {
        title: "wgpu | Camera fly".into(),
        ..framework::Settings::default()
    })
}
