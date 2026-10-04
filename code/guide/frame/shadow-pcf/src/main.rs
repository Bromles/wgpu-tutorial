use std::error::Error;

use shadow_pcf::sample::ShadowPcf;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    framework::run::<ShadowPcf>(framework::Settings {
        title: "wgpu | Shadow PCF".into(),
        ..framework::Settings::default()
    })
}
