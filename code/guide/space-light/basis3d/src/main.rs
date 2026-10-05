use std::error::Error;

use basis3d::sample::Basis3d;
use framework::{Settings, run};
use tracing::Level;
use tracing_subscriber::fmt;

fn main() -> Result<(), Box<dyn Error>> {
    fmt()
        .with_max_level(Level::INFO)
        .init();
    run::<Basis3d>(Settings {
        title: "wgpu | Basis 3D".into(),
        ..Settings::default()
    })
}
