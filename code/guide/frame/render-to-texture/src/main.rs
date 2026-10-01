use std::error::Error;

use render_to_texture::sample::RenderToTexture;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    shell::run::<RenderToTexture>(shell::Settings {
        title: "wgpu | Render to texture".into(),
        ..shell::Settings::default()
    })
}
