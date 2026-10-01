use std::error::Error;

use vertex_pulling::sample::VertexPulling;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    shell::run::<VertexPulling>(shell::Settings {
        title: "wgpu | Vertex pulling".into(),
        ..shell::Settings::default()
    })
}
