use std::error::Error;

use indexed_geometry::sample::IndexedGeometry;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    framework::run::<IndexedGeometry>(framework::Settings {
        title: "wgpu | Indexed geometry".into(),
        ..framework::Settings::default()
    })
}
