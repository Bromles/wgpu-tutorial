use std::error::Error;

use srgb_mixing::sample::SrgbMixing;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    framework::run::<SrgbMixing>(framework::Settings {
        title: "wgpu | sRGB mixing".into(),
        ..framework::Settings::default()
    })
}
