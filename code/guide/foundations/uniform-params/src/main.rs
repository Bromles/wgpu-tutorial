use std::error::Error;

use framework::{Settings, run};
use tracing::Level;
use tracing_subscriber::fmt;
use uniform_params::sample::UniformParams;

fn main() -> Result<(), Box<dyn Error>> {
    fmt()
        .with_max_level(Level::INFO)
        .init();
    run::<UniformParams>(Settings {
        title: "wgpu | Uniform params".into(),
        ..Settings::default()
    })
}
