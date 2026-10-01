use std::error::Error;

use srgb_mixing::sample::SrgbMixing;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    shell::run::<SrgbMixing>(shell::Settings {
        title: "wgpu | sRGB mixing".into(),
        ..shell::Settings::default()
    })
}
