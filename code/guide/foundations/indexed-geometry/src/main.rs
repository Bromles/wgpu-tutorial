use std::error::Error;

use framework::{Settings, run};
use tracing::Level;
use tracing_subscriber::fmt;
use indexed_geometry::sample::IndexedGeometry;

fn main() -> Result<(), Box<dyn Error>> {
    fmt()
        .with_max_level(Level::INFO)
        .init();
    run::<IndexedGeometry>(Settings {
        title: "wgpu | Indexed geometry".into(),
        ..Settings::default()
    })
}
