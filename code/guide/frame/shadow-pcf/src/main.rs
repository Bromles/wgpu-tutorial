use std::error::Error;

use framework::{Settings, run};
use shadow_pcf::sample::ShadowPcf;
use tracing::Level;
use tracing_subscriber::fmt;

fn main() -> Result<(), Box<dyn Error>> {
    fmt()
        .with_max_level(Level::INFO)
        .init();
    run::<ShadowPcf>(Settings {
        title: "wgpu | Shadow PCF".into(),
        ..Settings::default()
    })
}
