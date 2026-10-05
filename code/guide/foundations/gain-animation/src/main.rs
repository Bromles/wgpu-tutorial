use std::error::Error;

use framework::{Settings, run};
use tracing::Level;
use tracing_subscriber::fmt;
use gain_animation::sample::GainAnimation;

fn main() -> Result<(), Box<dyn Error>> {
    fmt()
        .with_max_level(Level::INFO)
        .init();
    run::<GainAnimation>(Settings {
        title: "wgpu | Gain animation".into(),
        ..Settings::default()
    })
}
