use std::error::Error;

use framework::{Settings, run};
use lambert::sample::Lambert;
use tracing::Level;
use tracing_subscriber::fmt;

fn main() -> Result<(), Box<dyn Error>> {
    fmt()
        .with_max_level(Level::INFO)
        .init();
    run::<Lambert>(Settings {
        title: "wgpu | Lambert".into(),
        ..Settings::default()
    })
}
