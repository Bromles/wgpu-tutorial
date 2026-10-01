use std::error::Error;

use blend_order::sample::BlendOrder;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    shell::run::<BlendOrder>(shell::Settings {
        title: "wgpu | Blend order".into(),
        ..shell::Settings::default()
    })
}
