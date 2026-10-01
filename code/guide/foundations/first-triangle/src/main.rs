use first_triangle::Sample;
use std::{
    error::Error,
    sync::Arc,
    time::{Duration, Instant},
};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

struct Context {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    sample: Sample,
    needs_configure: bool,
}

impl Context {
    fn new(window: Arc<Window>) -> Result<Self, Box<dyn Error>> {
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
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("First triangle device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            }))?;
        device.on_uncaptured_error(Arc::new(|error| {
            tracing::error!(%error, "Unrecoverable GPU error");
            std::process::exit(1);
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
        let sample = Sample::new(&device, format);
        Ok(Self {
            surface,
            device,
            queue,
            config,
            sample,
            needs_configure: true,
        })
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

    fn render(&self, window: &Window, frame: wgpu::SurfaceTexture) {
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("First triangle surface view"),
            ..Default::default()
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("First triangle encoder"),
            });
        self.sample.draw(&mut encoder, &view);
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

#[derive(Default)]
struct App {
    window: Option<Arc<Window>>,
    context: Option<Context>,
    active: bool,
    occluded: bool,
    size: PhysicalSize<u32>,
    retry_at: Option<Instant>,
    failure: Option<String>,
}

impl App {
    fn redraw(&mut self) -> Result<(), Box<dyn Error>> {
        if !self.active || self.occluded || self.size.width == 0 || self.size.height == 0 {
            self.retry_at = None;
            return Ok(());
        }
        if self
            .retry_at
            .is_some_and(|deadline| Instant::now() < deadline)
        {
            return Ok(());
        }
        self.retry_at = None;
        let window = self.window.as_ref().unwrap();
        if self.context.is_none() {
            self.context = Some(Context::new(window.clone())?);
        }
        let context = self.context.as_mut().unwrap();
        context.resize(self.size)?;
        match context.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => context.render(window, frame),
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                context.render(window, frame);
                // Present consumes the frame before any later configure.
                context.needs_configure = true;
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                context.needs_configure = true;
                self.retry_at = Some(Instant::now() + Duration::from_millis(100));
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                // wgpu 30: full context rebuild on the next redraw.
                self.context = None;
                self.retry_at = Some(Instant::now() + Duration::from_millis(100));
            }
            wgpu::CurrentSurfaceTexture::Timeout => {
                self.retry_at = Some(Instant::now() + Duration::from_millis(100));
            }
            wgpu::CurrentSurfaceTexture::Occluded => {
                // No timer; platforms without Occluded events recover via OS redraw.
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err("Surface acquisition failed validation; see GPU diagnostics".into());
            }
        }
        Ok(())
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.active {
            return;
        }
        if self.window.is_none() {
            match event_loop.create_window(
                Window::default_attributes()
                    .with_title("wgpu | First triangle")
                    .with_inner_size(PhysicalSize::new(800, 600)),
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
        self.retry_at = None;
        event_loop.set_control_flow(ControlFlow::Wait);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let Some(window) = &self.window else {
            return;
        };
        if window.id() != id {
            return;
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
                self.size = window.inner_size();
                if let Err(error) = self.redraw() {
                    tracing::error!(%error, "Rendering stopped");
                    self.failure = Some(error.to_string());
                    self.retry_at = None;
                    event_loop.exit();
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

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    let event_loop = EventLoop::new()?;
    let mut app = App::default();
    event_loop.run_app(&mut app)?;
    if let Some(error) = app.failure {
        return Err(error.into());
    }
    Ok(())
}
