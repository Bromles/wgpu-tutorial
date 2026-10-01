use std::error::Error;

use texture_load::sample::TextureLoad;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    shell::run::<TextureLoad>(shell::Settings {
        title: "wgpu | Texture load".into(),
        ..shell::Settings::default()
    })
}
