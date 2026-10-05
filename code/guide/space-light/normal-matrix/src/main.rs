use std::error::Error;

use framework::{Settings, run};
use normal_matrix::sample::NormalMatrix;
use tracing::Level;
use tracing_subscriber::fmt;

fn main() -> Result<(), Box<dyn Error>> {
    fmt().with_max_level(Level::INFO).init();
    run::<NormalMatrix>(Settings {
        title: "wgpu | Normal matrix".into(),
        ..Settings::default()
    })
}
