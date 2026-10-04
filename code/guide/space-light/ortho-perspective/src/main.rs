use std::error::Error;

use ortho_perspective::sample::OrthoPerspective;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    framework::run::<OrthoPerspective>(framework::Settings {
        title: "wgpu | Ortho perspective".into(),
        ..framework::Settings::default()
    })
}
