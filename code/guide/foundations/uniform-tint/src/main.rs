use std::error::Error;

use framework::{Settings, run};
use tracing::Level;
use tracing_subscriber::fmt;
use uniform_tint::sample::UniformTint;

fn main() -> Result<(), Box<dyn Error>> {
    fmt()
        .with_max_level(Level::INFO)
        .init();
    run::<UniformTint>(Settings {
        title: "wgpu | Uniform tint".into(),
        ..Settings::default()
    })
}
