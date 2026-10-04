use std::error::Error;

use vertex_fetch::sample::VertexFetch;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    framework::run::<VertexFetch>(framework::Settings {
        title: "wgpu | Vertex fetch".into(),
        ..framework::Settings::default()
    })
}
