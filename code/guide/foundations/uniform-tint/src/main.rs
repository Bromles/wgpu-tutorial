use std::error::Error;

use uniform_tint::sample::UniformTint;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    framework::run::<UniformTint>(framework::Settings {
        title: "wgpu | Uniform tint".into(),
        ..framework::Settings::default()
    })
}
