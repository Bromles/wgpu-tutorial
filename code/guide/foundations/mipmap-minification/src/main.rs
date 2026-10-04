use std::error::Error;

use mipmap_minification::sample::MipmapMinification;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    framework::run::<MipmapMinification>(framework::Settings {
        title: "wgpu | Mipmap minification".into(),
        ..framework::Settings::default()
    })
}
