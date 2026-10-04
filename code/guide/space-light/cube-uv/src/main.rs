use std::error::Error;

use cube_uv::sample::CubeUv;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    framework::run::<CubeUv>(framework::Settings {
        title: "wgpu | Cube UV".into(),
        ..framework::Settings::default()
    })
}
