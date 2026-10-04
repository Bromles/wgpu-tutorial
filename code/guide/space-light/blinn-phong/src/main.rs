use std::error::Error;

use blinn_phong::sample::BlinnPhong;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    framework::run::<BlinnPhong>(framework::Settings {
        title: "wgpu | Blinn-Phong".into(),
        ..framework::Settings::default()
    })
}
