use std::error::Error;

use ortho_perspective::sample::OrthoPerspective;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    shell::run::<OrthoPerspective>(shell::Settings {
        title: "wgpu | Ortho perspective".into(),
        ..shell::Settings::default()
    })
}
