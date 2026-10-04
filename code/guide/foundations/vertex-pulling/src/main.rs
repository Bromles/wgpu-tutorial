use std::error::Error;

use vertex_pulling::sample::VertexPulling;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    framework::run::<VertexPulling>(framework::Settings {
        title: "wgpu | Vertex pulling".into(),
        ..framework::Settings::default()
    })
}
