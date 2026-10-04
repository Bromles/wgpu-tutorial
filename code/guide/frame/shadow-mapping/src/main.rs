use std::error::Error;

use shadow_mapping::sample::ShadowMapping;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    framework::run::<ShadowMapping>(framework::Settings {
        title: "wgpu | Shadow mapping".into(),
        ..framework::Settings::default()
    })
}
