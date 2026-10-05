use std::error::Error;

use framework::{Settings, run};
use light_point::sample::LightPoint;
use tracing::Level;
use tracing_subscriber::fmt;

fn main() -> Result<(), Box<dyn Error>> {
    fmt()
        .with_max_level(Level::INFO)
        .init();
    run::<LightPoint>(Settings {
        title: "wgpu | Point light".into(),
        ..Settings::default()
    })
}
