use std::error::Error;

use image_pipeline::sample::ImagePipeline;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    framework::run::<ImagePipeline>(framework::Settings {
        title: "wgpu | Image pipeline".into(),
        ..framework::Settings::default()
    })
}
