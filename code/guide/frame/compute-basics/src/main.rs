use std::error::Error;

use framework::{Settings, run};
use tracing::Level;
use tracing_subscriber::fmt;
use compute_basics::sample::ComputeBasics;

fn main() -> Result<(), Box<dyn Error>> {
    fmt()
        .with_max_level(Level::INFO)
        .init();
    run::<ComputeBasics>(Settings {
        title: "wgpu | Compute basics".into(),
        ..Settings::default()
    })
}
