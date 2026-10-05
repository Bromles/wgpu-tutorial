use std::error::Error;

use framework::{Settings, run};
use tracing::Level;
use tracing_subscriber::fmt;
use light_list::sample::LightList;

fn main() -> Result<(), Box<dyn Error>> {
    fmt()
        .with_max_level(Level::INFO)
        .init();
    run::<LightList>(Settings {
        title: "wgpu | Light list".into(),
        ..Settings::default()
    })
}
