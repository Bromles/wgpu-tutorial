use std::error::Error;

use compute_basics::sample::ComputeBasics;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    framework::run::<ComputeBasics>(framework::Settings {
        title: "wgpu | Compute basics".into(),
        ..framework::Settings::default()
    })
}
