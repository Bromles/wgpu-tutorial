use std::error::Error;

use texture_load::sample::TextureLoad;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    framework::run::<TextureLoad>(framework::Settings {
        title: "wgpu | Texture load".into(),
        ..framework::Settings::default()
    })
}
