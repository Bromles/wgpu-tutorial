use std::error::Error;

use hdr_output::sample::HdrOutput;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    framework::run::<HdrOutput>(framework::Settings {
        title: "wgpu | HDR output".into(),
        ..framework::Settings::default()
    })
}
