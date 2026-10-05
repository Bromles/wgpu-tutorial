use std::error::Error;

use framework::{Settings, run};
use render_to_texture::sample::RenderToTexture;
use tracing::Level;
use tracing_subscriber::fmt;

fn main() -> Result<(), Box<dyn Error>> {
    fmt()
        .with_max_level(Level::INFO)
        .init();
    run::<RenderToTexture>(Settings {
        title: "wgpu | Render to texture".into(),
        ..Settings::default()
    })
}
