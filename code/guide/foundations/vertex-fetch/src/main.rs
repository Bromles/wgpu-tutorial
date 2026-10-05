use std::error::Error;

use framework::{Settings, run};
use tracing::Level;
use tracing_subscriber::fmt;
use vertex_fetch::sample::VertexFetch;

fn main() -> Result<(), Box<dyn Error>> {
    fmt()
        .with_max_level(Level::INFO)
        .init();
    run::<VertexFetch>(Settings {
        title: "wgpu | Vertex fetch".into(),
        ..Settings::default()
    })
}
