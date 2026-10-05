use std::error::Error;

use framework::{Settings, run};
use tracing::Level;
use tracing_subscriber::fmt;
use vertex_colors::sample::VertexColors;

fn main() -> Result<(), Box<dyn Error>> {
    fmt()
        .with_max_level(Level::INFO)
        .init();
    run::<VertexColors>(Settings {
        title: "wgpu | Vertex colors".into(),
        ..Settings::default()
    })
}
