use std::error::Error;

use mipmap_minification::sample::MipmapMinification;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    shell::run::<MipmapMinification>(shell::Settings {
        title: "wgpu | Mipmap minification".into(),
        ..shell::Settings::default()
    })
}
