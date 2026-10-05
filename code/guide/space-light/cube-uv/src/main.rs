use std::error::Error;

use cube_uv::sample::CubeUv;
use framework::{Settings, run};
use tracing::Level;
use tracing_subscriber::fmt;

fn main() -> Result<(), Box<dyn Error>> {
    fmt()
        .with_max_level(Level::INFO)
        .init();
    run::<CubeUv>(Settings {
        title: "wgpu | Cube UV".into(),
        ..Settings::default()
    })
}
