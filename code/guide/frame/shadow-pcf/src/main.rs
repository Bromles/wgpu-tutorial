use std::error::Error;

use shadow_pcf::sample::ShadowPcf;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    shell::run::<ShadowPcf>(shell::Settings {
        title: "wgpu | Shadow PCF".into(),
        ..shell::Settings::default()
    })
}
