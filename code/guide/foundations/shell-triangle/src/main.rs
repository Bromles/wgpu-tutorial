use std::error::Error;

use shell_triangle::sample::Triangle;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    shell::run::<Triangle>(shell::Settings {
        title: "wgpu | Shell triangle".into(),
        ..shell::Settings::default()
    })
}
