use std::error::Error;

use framework::{Settings, run};
use srgb_mixing::sample::SrgbMixing;
use tracing::Level;
use tracing_subscriber::fmt;

fn main() -> Result<(), Box<dyn Error>> {
    fmt()
        .with_max_level(Level::INFO)
        .init();
    run::<SrgbMixing>(Settings {
        title: "wgpu | sRGB mixing".into(),
        ..Settings::default()
    })
}
