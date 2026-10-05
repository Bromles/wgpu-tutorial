use std::error::Error;

use framework::{Settings, run};
use tracing::Level;
use tracing_subscriber::fmt;
use texture_sampling::sample::TextureSampling;

fn main() -> Result<(), Box<dyn Error>> {
    fmt()
        .with_max_level(Level::INFO)
        .init();
    run::<TextureSampling>(Settings {
        title: "wgpu | Texture sampling".into(),
        ..Settings::default()
    })
}
