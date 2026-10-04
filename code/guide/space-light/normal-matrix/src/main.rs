use std::error::Error;

use normal_matrix::sample::NormalMatrix;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    framework::run::<NormalMatrix>(framework::Settings {
        title: "wgpu | Normal matrix".into(),
        ..framework::Settings::default()
    })
}
