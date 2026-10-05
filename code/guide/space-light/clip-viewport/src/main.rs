use std::error::Error;

use clip_viewport::sample::ClipViewport;
use framework::{Settings, run};
use tracing::Level;
use tracing_subscriber::fmt;

fn main() -> Result<(), Box<dyn Error>> {
    fmt()
        .with_max_level(Level::INFO)
        .init();
    run::<ClipViewport>(Settings {
        title: "wgpu | Clip vs viewport".into(),
        ..Settings::default()
    })
}
