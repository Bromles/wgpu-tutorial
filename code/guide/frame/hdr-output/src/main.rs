use std::error::Error;

use framework::{Settings, run};
use tracing::Level;
use tracing_subscriber::fmt;
use hdr_output::sample::HdrOutput;

fn main() -> Result<(), Box<dyn Error>> {
    fmt()
        .with_max_level(Level::INFO)
        .init();
    run::<HdrOutput>(Settings {
        title: "wgpu | HDR output".into(),
        ..Settings::default()
    })
}
