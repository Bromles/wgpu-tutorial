use std::error::Error;

use blend_over::sample::BlendOver;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    framework::run::<BlendOver>(framework::Settings {
        title: "wgpu | Blend over".into(),
        ..framework::Settings::default()
    })
}
