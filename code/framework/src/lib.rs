//! Frozen minimal window framework shared by the foundation snapshots.
//! Owns window/surface lifecycle, acquire/submit/present, resize, and raw
//! event forwarding; device requirements arrive via [`Settings`].
use std::sync::Arc;

use winit::dpi::PhysicalSize;


mod app;
mod context;
mod gpu;
mod sample;
mod settings;

pub use gpu::Gpu;
pub use sample::Sample;
pub use settings::Settings;

use std::error::Error;

use winit::event_loop::EventLoop;

/// Runs the framework with the chapter sample until the window closes.
pub fn run<S: Sample>(settings: Settings) -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;
    let shared_failure: context::SharedFailure = Arc::default();
    let mut app = app::App::<S> {
        settings,
        window: None,
        context: None,
        sample: None,
        active: false,
        occluded: false,
        size: PhysicalSize::new(0, 0),
        retry_at: None,
        failure: None,
        shared_failure: shared_failure.clone(),
    };
    event_loop.run_app(&mut app)?;
    if let Some(error) = app.failure {
        return Err(error.into());
    }
    Ok(())
}
