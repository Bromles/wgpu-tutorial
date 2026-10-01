use std::error::Error;

use depth_culling::sample::DepthCulling;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    shell::run::<DepthCulling>(shell::Settings {
        title: "wgpu | Depth culling".into(),
        ..shell::Settings::default()
    })
}
