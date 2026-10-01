use std::error::Error;

use image_pipeline::sample::ImagePipeline;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    shell::run::<ImagePipeline>(shell::Settings {
        title: "wgpu | Image pipeline".into(),
        ..shell::Settings::default()
    })
}
