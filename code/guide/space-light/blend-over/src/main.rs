use std::error::Error;

use blend_over::sample::BlendOver;
use framework::{Settings, run};
use tracing::Level;
use tracing_subscriber::fmt;

fn main() -> Result<(), Box<dyn Error>> {
    fmt()
        .with_max_level(Level::INFO)
        .init();
    run::<BlendOver>(Settings {
        title: "wgpu | Over".into(),
        ..Settings::default()
    })
}
