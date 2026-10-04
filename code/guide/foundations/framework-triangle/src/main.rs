use std::error::Error;

use framework_triangle::sample::Triangle;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    framework::run::<Triangle>(framework::Settings {
        title: "wgpu | Framework triangle".into(),
        ..framework::Settings::default()
    })
}
