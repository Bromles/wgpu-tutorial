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

use wgpu::Surface;
use wgpu::Device;
use wgpu::Queue;
use wgpu::SurfaceConfiguration;
use wgpu::Instance;
use wgpu::InstanceDescriptor;
use wgpu::Backends;
use pollster::block_on;
use wgpu::RequestAdapterOptions;
use wgpu::DeviceDescriptor;
use wgpu::Features;
use wgpu::Limits;
use tracing::error;
use std::process::exit;
use wgpu::TextureFormat;
use wgpu::PresentMode;
use tracing::info;
use wgpu::TextureUsages;
use wgpu::SurfaceColorSpace;
use wgpu::SurfaceTexture;
use wgpu::TextureViewDescriptor;
use wgpu::CommandEncoderDescriptor;
use wgpu::RenderPassDescriptor;
use wgpu::RenderPassColorAttachment;
use wgpu::Operations;
use wgpu::LoadOp;
use wgpu::Color;
use wgpu::StoreOp;
use wgpu::CurrentSurfaceTexture;
use tracing_subscriber::fmt;
use tracing::Level;
struct Context {
    surface: Surface<'static>,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    needs_configure: bool,
}

impl Context {
    fn new(window: Arc<Window>) -> Result<Self, Box<dyn Error>> {
        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::PRIMARY,
            ..InstanceDescriptor::new_without_display_handle()
        });
        let surface = instance.create_surface(window)?;
        let adapter = block_on(instance.request_adapter(&RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))?;
        let (device, queue) =
            block_on(adapter.request_device(&DeviceDescriptor {
                label: Some("Surface lifecycle device"),
                required_features: Features::empty(),
                required_limits: Limits::default(),
                ..Default::default()
            }))?;
        device.on_uncaptured_error(Arc::new(|error| {
            error!(%error, "Unrecoverable GPU error");
            exit(1);
        }));
        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(TextureFormat::is_srgb)
            .ok_or("This example requires an sRGB surface format")?;
        let alpha_mode = capabilities
            .alpha_modes
            .first()
            .copied()
            .ok_or("Surface has no supported alpha mode")?;
        if !capabilities
            .present_modes
            .contains(&PresentMode::Fifo)
        {
            return Err("Surface does not support FIFO presentation".into());
        }
        let info = adapter.get_info();
        info!(adapter = %info.name, backend = ?info.backend, ?format, "Selected GPU");
        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format,
            width: 0,
            height: 0,
            present_mode: PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode,
            view_formats: vec![],
            color_space: SurfaceColorSpace::Auto,
        };
        Ok(Self {
            surface,
            device,
            queue,
            config,
            needs_configure: true,
        })
    }

    fn resize(&mut self,
    size: PhysicalSize<u32>) -> Result<(), Box<dyn Error>> {
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
            info!(
                width = size.width,
                height = size.height,
                "Configured surface"
            );
        }
        Ok(())
    }

    fn render(&self,
    window: &Window,
    frame: SurfaceTexture) {
        let view = frame.texture.create_view(&TextureViewDescriptor {
            label: Some("Surface lifecycle surface view"),
            ..Default::default()
        });
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Surface lifecycle clear encoder"),
            });
        {
            let _pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Surface lifecycle clear pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color {
                            r: 0.5,
                            g: 0.5,
                            b: 0.5,
                            a: 1.0,
                        }),
                        store: StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                ..Default::default()
            });
        }
        self.queue.submit([encoder.finish()]);
        window.pre_present_notify();
        self.queue.present(frame);
        info!(
            width = self.config.width,
            height = self.config.height,
            "Rendered clear"
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
            CurrentSurfaceTexture::Success(frame) => context.render(window, frame),
            CurrentSurfaceTexture::Suboptimal(frame) => {
                context.render(window, frame);
                // Present consumes the frame before any later configure.
                context.needs_configure = true;
            }
            CurrentSurfaceTexture::Outdated => {
                context.needs_configure = true;
                self.retry_at = Some(Instant::now() + Duration::from_millis(100));
            }
            CurrentSurfaceTexture::Lost => {
                // wgpu 30: full context rebuild on the next redraw.
                self.context = None;
                self.retry_at = Some(Instant::now() + Duration::from_millis(100));
            }
            CurrentSurfaceTexture::Timeout => {
                self.retry_at = Some(Instant::now() + Duration::from_millis(100));
            }
            CurrentSurfaceTexture::Occluded => {
                // No timer; platforms without Occluded events recover via OS redraw.
            }
            CurrentSurfaceTexture::Validation => {
                return Err("Surface acquisition failed validation; see GPU diagnostics".into());
            }
        }
        Ok(())
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self,
    event_loop: &ActiveEventLoop) {
        if self.active {
            return;
        }
        if self.window.is_none() {
            match event_loop.create_window(
                Window::default_attributes()
                    .with_title("wgpu | Surface lifecycle")
                    .with_inner_size(PhysicalSize::new(800, 600)),
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
        self.retry_at = None;
        event_loop.set_control_flow(ControlFlow::Wait);
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
                    error!(%error, "Rendering stopped");
                    self.failure = Some(error.to_string());
                    self.retry_at = None;
                    event_loop.exit();
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

fn main() -> Result<(), Box<dyn Error>> {
    fmt()
        .with_max_level(Level::INFO)
        .init();
    let event_loop = EventLoop::new()?;
    let mut app = App::default();
    event_loop.run_app(&mut app)?;
    if let Some(error) = app.failure {
        return Err(error.into());
    }
    Ok(())
}
