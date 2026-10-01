use std::error::Error;

use blinn_phong::sample::BlinnPhong;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    shell::run::<BlinnPhong>(shell::Settings {
        title: "wgpu | Blinn-Phong".into(),
        ..shell::Settings::default()
    })
}
