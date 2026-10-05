use std::error::Error;

use framework::{Settings, run};
use shadow_mapping::sample::ShadowMapping;
use tracing::Level;
use tracing_subscriber::fmt;

fn main() -> Result<(), Box<dyn Error>> {
    fmt()
        .with_max_level(Level::INFO)
        .init();
    run::<ShadowMapping>(Settings {
        title: "wgpu | Shadow mapping".into(),
        ..Settings::default()
    })
}
