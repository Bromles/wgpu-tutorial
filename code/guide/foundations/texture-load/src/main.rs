use std::error::Error;

use framework::{Settings, run};
use texture_load::sample::TextureLoad;
use tracing::Level;
use tracing_subscriber::fmt;

fn main() -> Result<(), Box<dyn Error>> {
    fmt()
        .with_max_level(Level::INFO)
        .init();
    run::<TextureLoad>(Settings {
        title: "wgpu | Texture load".into(),
        ..Settings::default()
    })
}
