use std::error::Error;

use shadow_mapping::sample::ShadowMapping;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    shell::run::<ShadowMapping>(shell::Settings {
        title: "wgpu | Shadow mapping".into(),
        ..shell::Settings::default()
    })
}
