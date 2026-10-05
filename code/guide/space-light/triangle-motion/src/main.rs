use std::error::Error;

use framework::{Settings, run};
use tracing::Level;
use tracing_subscriber::fmt;
use triangle_motion::sample::TriangleMotion;

fn main() -> Result<(), Box<dyn Error>> {
    fmt()
        .with_max_level(Level::INFO)
        .init();
    run::<TriangleMotion>(Settings {
        title: "wgpu | Triangle motion".into(),
        ..Settings::default()
    })
}
