use std::error::Error;

use look_at::sample::LookAt;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    framework::run::<LookAt>(framework::Settings {
        title: "wgpu | Look-at".into(),
        ..framework::Settings::default()
    })
}
