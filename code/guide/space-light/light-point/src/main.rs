use std::error::Error;

use light_point::sample::LightPoint;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    shell::run::<LightPoint>(shell::Settings {
        title: "wgpu | Point and spot".into(),
        ..shell::Settings::default()
    })
}
