use std::error::Error;

use compute_basics::sample::ComputeBasics;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    shell::run::<ComputeBasics>(shell::Settings {
        title: "wgpu | Compute basics".into(),
        ..shell::Settings::default()
    })
}
