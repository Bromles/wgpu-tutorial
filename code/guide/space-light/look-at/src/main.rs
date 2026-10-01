use std::error::Error;

use look_at::sample::LookAt;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    shell::run::<LookAt>(shell::Settings {
        title: "wgpu | Look-at".into(),
        ..shell::Settings::default()
    })
}
