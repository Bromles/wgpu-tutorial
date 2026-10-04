use std::error::Error;

use clip_viewport::sample::ClipViewport;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    framework::run::<ClipViewport>(framework::Settings {
        title: "wgpu | Clip viewport".into(),
        ..framework::Settings::default()
    })
}
