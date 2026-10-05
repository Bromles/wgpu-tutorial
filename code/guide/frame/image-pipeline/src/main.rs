use std::error::Error;

use framework::{Settings, run};
use tracing::Level;
use tracing_subscriber::fmt;
use image_pipeline::sample::ImagePipeline;

fn main() -> Result<(), Box<dyn Error>> {
    fmt()
        .with_max_level(Level::INFO)
        .init();
    run::<ImagePipeline>(Settings {
        title: "wgpu | Image pipeline".into(),
        ..Settings::default()
    })
}
