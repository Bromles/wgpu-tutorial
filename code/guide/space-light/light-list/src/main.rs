use std::error::Error;

use light_list::sample::LightList;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    shell::run::<LightList>(shell::Settings {
        title: "wgpu | Light list".into(),
        ..shell::Settings::default()
    })
}
