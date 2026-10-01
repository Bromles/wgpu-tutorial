use std::error::Error;

use gain_animation::sample::GainAnimation;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    shell::run::<GainAnimation>(shell::Settings {
        title: "wgpu | Gain animation".into(),
        ..shell::Settings::default()
    })
}
