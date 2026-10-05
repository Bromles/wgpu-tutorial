use std::error::Error;

use framework::{Settings, run};
use tracing::Level;
use tracing_subscriber::fmt;
use vertex_pulling::sample::VertexPulling;

fn main() -> Result<(), Box<dyn Error>> {
    fmt()
        .with_max_level(Level::INFO)
        .init();
    run::<VertexPulling>(Settings {
        title: "wgpu | Vertex pulling".into(),
        ..Settings::default()
    })
}
