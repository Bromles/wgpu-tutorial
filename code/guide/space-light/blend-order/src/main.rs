use std::error::Error;

use blend_order::sample::BlendOrder;
use framework::{Settings, run};
use tracing::Level;
use tracing_subscriber::fmt;

fn main() -> Result<(), Box<dyn Error>> {
    fmt()
        .with_max_level(Level::INFO)
        .init();
    run::<BlendOrder>(Settings {
        title: "wgpu | Blend order".into(),
        ..Settings::default()
    })
}
