use std::error::Error;

use lambert::sample::Lambert;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    shell::run::<Lambert>(shell::Settings {
        title: "wgpu | Lambert".into(),
        ..shell::Settings::default()
    })
}
