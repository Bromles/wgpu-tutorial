use std::error::Error;

use depth_culling::sample::DepthCulling;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    framework::run::<DepthCulling>(framework::Settings {
        title: "wgpu | Depth culling".into(),
        ..framework::Settings::default()
    })
}
