use std::error::Error;

use triangle_motion::sample::TriangleMotion;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    shell::run::<TriangleMotion>(shell::Settings {
        title: "wgpu | Triangle motion".into(),
        ..shell::Settings::default()
    })
}
