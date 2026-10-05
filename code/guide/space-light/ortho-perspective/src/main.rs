use std::error::Error;

use framework::{Settings, run};
use ortho_perspective::sample::OrthoPerspective;
use tracing::Level;
use tracing_subscriber::fmt;

fn main() -> Result<(), Box<dyn Error>> {
    fmt()
        .with_max_level(Level::INFO)
        .init();
    run::<OrthoPerspective>(Settings {
        title: "wgpu | Ortho vs perspective".into(),
        ..Settings::default()
    })
}
