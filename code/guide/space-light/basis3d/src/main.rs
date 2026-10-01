use std::error::Error;

use basis3d::sample::Basis3d;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    shell::run::<Basis3d>(shell::Settings {
        title: "wgpu | Basis".into(),
        ..shell::Settings::default()
    })
}
