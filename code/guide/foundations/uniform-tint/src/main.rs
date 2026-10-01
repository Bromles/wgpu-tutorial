use std::error::Error;

use uniform_tint::sample::UniformTint;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    shell::run::<UniformTint>(shell::Settings {
        title: "wgpu | Uniform tint".into(),
        ..shell::Settings::default()
    })
}
