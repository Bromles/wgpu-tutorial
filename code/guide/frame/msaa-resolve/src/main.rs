use std::error::Error;

use framework::{Settings, run};
use msaa_resolve::sample::MsaaResolve;
use tracing::Level;
use tracing_subscriber::fmt;

fn main() -> Result<(), Box<dyn Error>> {
    fmt()
        .with_max_level(Level::INFO)
        .init();
    run::<MsaaResolve>(Settings {
        title: "wgpu | MSAA resolve".into(),
        ..Settings::default()
    })
}
