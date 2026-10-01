use std::error::Error;

use vertex_colors::sample::VertexColors;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    shell::run::<VertexColors>(shell::Settings {
        title: "wgpu | Vertex colors".into(),
        ..shell::Settings::default()
    })
}
