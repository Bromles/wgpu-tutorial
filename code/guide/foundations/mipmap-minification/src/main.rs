use std::error::Error;

use framework::{Settings, run};
use mipmap_minification::sample::MipmapMinification;
use tracing::Level;
use tracing_subscriber::fmt;

fn main() -> Result<(), Box<dyn Error>> {
    fmt()
        .with_max_level(Level::INFO)
        .init();
    run::<MipmapMinification>(Settings {
        title: "wgpu | Mipmap minification".into(),
        ..Settings::default()
    })
}
