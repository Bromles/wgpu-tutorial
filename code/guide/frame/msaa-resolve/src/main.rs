use std::error::Error;

use msaa_resolve::sample::MsaaResolve;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    framework::run::<MsaaResolve>(framework::Settings {
        title: "wgpu | MSAA resolve".into(),
        ..framework::Settings::default()
    })
}
