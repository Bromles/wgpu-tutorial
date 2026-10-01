use std::error::Error;

use blend_over::sample::BlendOver;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    shell::run::<BlendOver>(shell::Settings {
        title: "wgpu | Blend over".into(),
        ..shell::Settings::default()
    })
}
