use std::{
    error::Error,
    sync::Arc,
    time::{Duration, Instant},
};
use tracing::error;
use wgpu::CurrentSurfaceTexture;
use winit::event::DeviceId;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{DeviceEvent, ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

use crate::{
    context::{Context, SharedFailure},
    sample::Sample,
    settings::Settings,
};

pub(crate) struct App<S: Sample> {
    pub(crate) settings: Settings,
    pub(crate) window: Option<Arc<Window>>,
    pub(crate) context: Option<Context>,
    pub(crate) sample: Option<S>,
    pub(crate) active: bool,
    pub(crate) occluded: bool,
    pub(crate) size: PhysicalSize<u32>,
    pub(crate) retry_at: Option<Instant>,
    pub(crate) failure: Option<String>,
    pub(crate) shared_failure: SharedFailure,
}

impl<S: Sample> App<S> {
    /// One redraw attempt; returns `true` if the sample was created by this
    /// call (the caller then re-delivers the triggering event).
    fn redraw(&mut self) -> Result<bool, Box<dyn Error>> {
        // An uncaptured GPU error records asynchronously; pick it up here.
        if let Some(error) = self.shared_failure.lock().expect("failure lock").take() {
            return Err(error.into());
        }
        if !self.active || self.occluded || self.size.width == 0 || self.size.height == 0 {
            self.retry_at = None;
            return Ok(false);
        }
        if self
            .retry_at
            .is_some_and(|deadline| Instant::now() < deadline)
        {
            return Ok(false);
        }
        self.retry_at = None;
        let mut created = false;
        let window = self.window.as_ref().unwrap().clone();
        if self.context.is_none() {
            let context = Context::new(window.clone(), &self.settings, &self.shared_failure)?;
            let gpu = context.gpu();
            let mut sample = S::init(&gpu)?;
            // Resize contract: real size right after creation, before the first draw.
            sample.resize(self.size.width, self.size.height);
            self.sample = Some(sample);
            self.context = Some(context);
            created = true;
        }
        let context = self.context.as_mut().unwrap();
        context.resize(self.size)?;
        match context.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(frame) => {
                context.render(&window, frame, self.sample.as_mut().unwrap());
            }
            CurrentSurfaceTexture::Suboptimal(frame) => {
                context.render(&window, frame, self.sample.as_mut().unwrap());
                // Present consumes the frame before any later configure.
                context.needs_configure = true;
            }
            CurrentSurfaceTexture::Outdated => {
                context.needs_configure = true;
                self.retry_at = Some(Instant::now() + Duration::from_millis(100));
            }
            CurrentSurfaceTexture::Lost => {
                // wgpu 30: Lost requires full surface (and sample) recreation.
                self.context = None;
                self.sample = None;
                self.retry_at = Some(Instant::now() + Duration::from_millis(100));
            }
            CurrentSurfaceTexture::Timeout => {
                self.retry_at = Some(Instant::now() + Duration::from_millis(100));
            }
            CurrentSurfaceTexture::Occluded => {
                // No retry timer; platforms without Occluded events recover via OS redraw.
            }
            CurrentSurfaceTexture::Validation => {
                return Err("Surface acquisition failed validation; see GPU diagnostics".into());
            }
        }
        Ok(created)
    }
}

impl<S: Sample> ApplicationHandler for App<S> {
    fn resumed(&mut self,
    event_loop: &ActiveEventLoop) {
        if self.active {
            return;
        }
        if self.window.is_none() {
            match event_loop.create_window(
                Window::default_attributes()
                    .with_title(self.settings.title.clone())
                    .with_inner_size(PhysicalSize::new(
                        self.settings.inner_size.0,
                        self.settings.inner_size.1,
                    )),
            ) {
                Ok(window) => self.window = Some(Arc::new(window)),
                Err(error) => {
                    error!(%error, "Window creation failed");
                    self.failure = Some(error.to_string());
                    event_loop.exit();
                    return;
                }
            }
        }
        self.active = true;
        self.occluded = false;
        self.retry_at = None;
        let window = self.window.as_ref().unwrap();
        self.size = window.inner_size();
        window.request_redraw();
    }

    fn suspended(&mut self,
    event_loop: &ActiveEventLoop) {
        self.active = false;
        self.context = None;
        self.sample = None;
        self.retry_at = None;
        event_loop.set_control_flow(ControlFlow::Wait);
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        // Raw forwarding only: no filtering or interpretation.
        if let Some(sample) = &mut self.sample {
            sample.device_event(&event);
        }
    }

    fn window_event(&mut self,
    event_loop: &ActiveEventLoop,
    id: WindowId,
    event: WindowEvent) {
        let Some(window) = &self.window else {
            return;
        };
        if window.id() != id {
            return;
        }
        // Raw forwarding first: chapter-local code reacts to its own events.
        if let Some(sample) = &mut self.sample {
            sample.window_event(window, &event);
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed
                    && event.physical_key == PhysicalKey::Code(KeyCode::Escape) =>
            {
                event_loop.exit()
            }
            WindowEvent::Resized(size) => {
                self.size = size;
                self.retry_at = None;
                if let Some(context) = &mut self.context {
                    context.needs_configure = true;
                }
                // Raw forwarding still delivers Resized; the resize callback
                // is filtered to nonzero sizes (zero = minimized).
                if size.width != 0
                    && size.height != 0
                    && let Some(sample) = &mut self.sample
                {
                    sample.resize(size.width, size.height);
                }
                if self.active && !self.occluded && size.width != 0 && size.height != 0 {
                    window.request_redraw();
                }
            }
            WindowEvent::Occluded(occluded) => {
                self.occluded = occluded;
                self.retry_at = None;
                if self.active && !occluded {
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                if !self.active {
                    return;
                }
                // Arc clone keeps the window usable across the &mut redraw call.
                let window = window.clone();
                self.size = window.inner_size();
                let created = match self.redraw() {
                    Ok(created) => created,
                    Err(error) => {
                        error!(%error, "Rendering stopped");
                        self.failure = Some(error.to_string());
                        self.retry_at = None;
                        event_loop.exit();
                        return;
                    }
                };
                // Newborn sample missed the forwarding above; re-deliver or animation stalls.
                if created && let Some(sample) = &mut self.sample {
                    sample.window_event(&window, &event);
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self,
    event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Wait);
        if !self.active || self.occluded || self.size.width == 0 || self.size.height == 0 {
            self.retry_at = None;
            return;
        }
        if let Some(deadline) = self.retry_at {
            if Instant::now() >= deadline {
                self.retry_at = None;
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            } else {
                event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
            }
        }
    }
}
