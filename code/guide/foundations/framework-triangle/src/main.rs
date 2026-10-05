use std::error::Error;

use framework::{Settings, run};
use framework_triangle::sample::Triangle;
use tracing::Level;
use tracing_subscriber::fmt;

fn main() -> Result<(), Box<dyn Error>> {
    fmt().with_max_level(Level::INFO).init();
    run::<Triangle>(Settings {
        title: "wgpu | Framework triangle".into(),
        ..Settings::default()
    })
}
