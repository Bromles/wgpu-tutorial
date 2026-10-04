use std::error::Error;

use texture_sampling::sample::TextureSampling;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    framework::run::<TextureSampling>(framework::Settings {
        title: "wgpu | Texture sampling".into(),
        ..framework::Settings::default()
    })
}
