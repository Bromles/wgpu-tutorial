use std::error::Error;

use uniform_params::sample::UniformParams;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    framework::run::<UniformParams>(framework::Settings {
        title: "wgpu | Uniform params".into(),
        ..framework::Settings::default()
    })
}
