use std::error::Error;

use vertex_fetch::sample::VertexFetch;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    shell::run::<VertexFetch>(shell::Settings {
        title: "wgpu | Vertex fetch".into(),
        ..shell::Settings::default()
    })
}
