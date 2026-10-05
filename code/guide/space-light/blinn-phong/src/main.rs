use std::error::Error;

use blinn_phong::sample::BlinnPhong;
use framework::{Settings, run};
use tracing::Level;
use tracing_subscriber::fmt;

fn main() -> Result<(), Box<dyn Error>> {
    fmt()
        .with_max_level(Level::INFO)
        .init();
    run::<BlinnPhong>(Settings {
        title: "wgpu | Blinn-Phong".into(),
        ..Settings::default()
    })
}
