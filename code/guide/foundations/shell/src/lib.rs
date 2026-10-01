//! Frozen minimal window shell shared by the foundation snapshots.
//! Owns window/surface lifecycle, acquire/submit/present, resize, and raw
//! event forwarding; device requirements arrive via [`Settings`].

use std::{
    error::Error,
    sync::Arc,
    time::{Duration, Instant},
};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{DeviceEvent, ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

/// Cloned device/queue handles handed to the sample every frame.
pub struct Gpu {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    /// sRGB surface format negotiated in chapter 03.
    pub format: wgpu::TextureFormat,
}

/// Chapter-local drawing code: the shell calls it, never the other way round.
pub trait Sample: 'static {
    /// Creates resources; called again after every surface loss, with the new device.
    fn init(gpu: &Gpu) -> Result<Self, Box<dyn Error>>
    where
        Self: Sized;

    /// Records one frame into `encoder`; the shell submits, presents, owns the frame.
    fn draw(&mut self, gpu: &Gpu, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView);

    /// Called right after `init`, before the first `draw`, and on every resize;
    /// zero sizes are filtered out by the shell. Optional.
    fn resize(&mut self, _width: u32, _height: u32) {}

    /// Raw window-event forwarding while a context exists; during the ~100 ms
    /// recovery after a surface loss (and while suspended) the sample is
    /// absent and its events are dropped. Optional.
    fn window_event(&mut self, _window: &Window, _event: &WindowEvent) {}

    /// Raw device-event forwarding (e.g. relative mouse motion) while a
    /// context exists. Optional.
    fn device_event(&mut self, _event: &DeviceEvent) {}
}

/// Example-supplied configuration, including device requirements.
pub struct Settings {
    pub title: String,
    pub inner_size: (u32, u32),
    pub device_descriptor: wgpu::DeviceDescriptor<'static>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            title: "wgpu".to_string(),
            inner_size: (800, 600),
            device_descriptor: wgpu::DeviceDescriptor {
                label: Some("Shell device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            },
        }
    }
}

struct Context {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    needs_configure: bool,
}

impl Context {
    fn new(
        window: Arc<Window>,
        settings: &Settings,
        shared_failure: &SharedFailure,
    ) -> Result<Self, Box<dyn Error>> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let surface = instance.create_surface(window)?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&settings.device_descriptor))?;
        // Record instead of exiting: the event loop keeps running until the
        // next redraw notices the failure and unwinds normally.
        let failure_sink = shared_failure.clone();
        device.on_uncaptured_error(Arc::new(move |error| {
            tracing::error!(%error, "Unrecoverable GPU error");
            let mut slot = failure_sink.lock().expect("failure lock");
            if slot.is_none() {
                *slot = Some(format!("Unrecoverable GPU error: {error}"));
            }
        }));
        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .ok_or("This example requires an sRGB surface format")?;
        let alpha_mode = capabilities
            .alpha_modes
            .first()
            .copied()
            .ok_or("Surface has no supported alpha mode")?;
        if !capabilities
            .present_modes
            .contains(&wgpu::PresentMode::Fifo)
        {
            return Err("Surface does not support FIFO presentation".into());
        }
        let info = adapter.get_info();
        tracing::info!(adapter = %info.name, backend = ?info.backend, ?format, "Selected GPU");
        // Width/height are placeholders only; resize configures the actual nonzero size.
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: 0,
            height: 0,
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode,
            view_formats: vec![],
            color_space: wgpu::SurfaceColorSpace::Auto,
        };
        Ok(Self {
            surface,
            device,
            queue,
            config,
            needs_configure: true,
        })
    }

    fn gpu(&self) -> Gpu {
        Gpu {
            device: self.device.clone(),
            queue: self.queue.clone(),
            format: self.config.format,
        }
    }

    fn resize(&mut self, size: PhysicalSize<u32>) -> Result<(), Box<dyn Error>> {
        if size.width == 0 || size.height == 0 {
            return Ok(());
        }
        let limit = self.device.limits().max_texture_dimension_2d;
        if size.width > limit || size.height > limit {
            return Err("Window size exceeds the requested device texture limit".into());
        }
        if self.needs_configure
            || (self.config.width, self.config.height) != (size.width, size.height)
        {
            self.config.width = size.width;
            self.config.height = size.height;
            self.surface.configure(&self.device, &self.config);
            self.needs_configure = false;
            tracing::info!(
                width = size.width,
                height = size.height,
                "Configured surface"
            );
        }
        Ok(())
    }

    fn render<S: Sample>(&mut self, window: &Window, frame: wgpu::SurfaceTexture, sample: &mut S) {
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Shell surface view"),
            ..Default::default()
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Shell encoder"),
            });
        sample.draw(&self.gpu(), &mut encoder, &view);
        self.queue.submit([encoder.finish()]);
        window.pre_present_notify();
        self.queue.present(frame);
        tracing::info!(
            width = self.config.width,
            height = self.config.height,
            "Rendered frame"
        );
    }
}

/// Terminal failure shared with the uncaptured-error handler, which runs on
/// an arbitrary thread and cannot reach `App` directly.
type SharedFailure = Arc<std::sync::Mutex<Option<String>>>;

struct App<S: Sample> {
    settings: Settings,
    window: Option<Arc<Window>>,
    context: Option<Context>,
    sample: Option<S>,
    active: bool,
    occluded: bool,
    size: PhysicalSize<u32>,
    retry_at: Option<Instant>,
    failure: Option<String>,
    shared_failure: SharedFailure,
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
            wgpu::CurrentSurfaceTexture::Success(frame) => {
                context.render(&window, frame, self.sample.as_mut().unwrap());
            }
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                context.render(&window, frame, self.sample.as_mut().unwrap());
                // Present consumes the frame before any later configure.
                context.needs_configure = true;
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                context.needs_configure = true;
                self.retry_at = Some(Instant::now() + Duration::from_millis(100));
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                // wgpu 30: Lost requires full surface (and sample) recreation.
                self.context = None;
                self.sample = None;
                self.retry_at = Some(Instant::now() + Duration::from_millis(100));
            }
            wgpu::CurrentSurfaceTexture::Timeout => {
                self.retry_at = Some(Instant::now() + Duration::from_millis(100));
            }
            wgpu::CurrentSurfaceTexture::Occluded => {
                // No retry timer; platforms without Occluded events recover via OS redraw.
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err("Surface acquisition failed validation; see GPU diagnostics".into());
            }
        }
        Ok(created)
    }
}

impl<S: Sample> ApplicationHandler for App<S> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
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
                    tracing::error!(%error, "Window creation failed");
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

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        self.active = false;
        self.context = None;
        self.sample = None;
        self.retry_at = None;
        event_loop.set_control_flow(ControlFlow::Wait);
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        // Raw forwarding only: no filtering or interpretation.
        if let Some(sample) = &mut self.sample {
            sample.device_event(&event);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
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
                if size.width != 0 && size.height != 0 {
                    if let Some(sample) = &mut self.sample {
                        sample.resize(size.width, size.height);
                    }
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
                        tracing::error!(%error, "Rendering stopped");
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

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
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

/// Runs the shell with the chapter sample until the window closes.
pub fn run<S: Sample>(settings: Settings) -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;
    let shared_failure: SharedFailure = Arc::default();
    let mut app = App::<S> {
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
