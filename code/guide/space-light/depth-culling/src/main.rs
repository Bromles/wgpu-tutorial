use std::error::Error;

use framework::{Settings, run};
use tracing::Level;
use tracing_subscriber::fmt;
use depth_culling::sample::DepthCulling;

fn main() -> Result<(), Box<dyn Error>> {
    fmt()
        .with_max_level(Level::INFO)
        .init();
    run::<DepthCulling>(Settings {
        title: "wgpu | Depth culling".into(),
        ..Settings::default()
    })
}
