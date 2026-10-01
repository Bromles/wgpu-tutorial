use std::error::Error;

use matrix_compose::sample::MatrixCompose;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    shell::run::<MatrixCompose>(shell::Settings {
        title: "wgpu | Matrix compose".into(),
        ..shell::Settings::default()
    })
}
