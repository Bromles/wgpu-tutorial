use std::error::Error;

use framework::{Settings, run};
use tracing::Level;
use tracing_subscriber::fmt;
use matrix_compose::sample::MatrixCompose;

fn main() -> Result<(), Box<dyn Error>> {
    fmt()
        .with_max_level(Level::INFO)
        .init();
    run::<MatrixCompose>(Settings {
        title: "wgpu | Matrix compose".into(),
        ..Settings::default()
    })
}
