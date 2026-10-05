use std::{error::Error, sync::Arc};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
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
use wgpu::CurrentSurfaceTexture;
use wgpu::TextureViewDescriptor;
use wgpu::CommandEncoderDescriptor;
use wgpu::RenderPassDescriptor;
use wgpu::RenderPassColorAttachment;
use wgpu::Operations;
use wgpu::LoadOp;
use wgpu::Color;
use wgpu::StoreOp;
use tracing_subscriber::fmt;
use tracing::Level;
struct Context {
    surface: Surface<'static>,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
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
                label: Some("First frame device"),
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
        // Width/height are placeholders only; resize configures the actual nonzero size.
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
        if (self.config.width, self.config.height) != (size.width, size.height) {
            self.config.width = size.width;
            self.config.height = size.height;
            self.surface.configure(&self.device, &self.config);
            info!(
                width = size.width,
                height = size.height,
                "Configured surface"
            );
        }
        Ok(())
    }

    fn render(&self,
    window: &Window) -> Result<(), Box<dyn Error>> {
        let frame = match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(frame)
            | CurrentSurfaceTexture::Suboptimal(frame) => frame,
            CurrentSurfaceTexture::Timeout => {
                return Err(
                    "Surface acquisition timed out; restart first-frame or use surface-lifecycle for delayed retries"
                        .into(),
                );
            }
            CurrentSurfaceTexture::Occluded => return Ok(()),
            CurrentSurfaceTexture::Outdated | CurrentSurfaceTexture::Lost => {
                return Err(
                    "Surface needs recovery; restart first-frame or use the surface-lifecycle example".into(),
                );
            }
            CurrentSurfaceTexture::Validation => {
                return Err("Surface acquisition failed validation; see GPU diagnostics".into());
            }
        };
        let view = frame.texture.create_view(&TextureViewDescriptor {
            label: Some("First frame surface view"),
            ..Default::default()
        });
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("First frame clear encoder"),
            });
        {
            let _pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("First frame clear pass"),
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
        Ok(())
    }
}

#[derive(Default)]
struct App {
    window: Option<Arc<Window>>,
    context: Option<Context>,
    failure: Option<String>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self,
    event_loop: &ActiveEventLoop) {
        if self.context.is_some() {
            return;
        }
        let result = (|| -> Result<(), Box<dyn Error>> {
            if self.window.is_none() {
                self.window = Some(Arc::new(
                    event_loop.create_window(
                        Window::default_attributes()
                            .with_title("wgpu | First frame")
                            .with_inner_size(PhysicalSize::new(800, 600))
                            .with_resizable(false),
                    )?,
                ));
            }
            let window = self.window.as_ref().unwrap();
            self.context = Some(Context::new(window.clone())?);
            window.request_redraw();
            Ok(())
        })();
        if let Err(error) = result {
            error!(%error, "Initialization failed");
            self.failure = Some(error.to_string());
            event_loop.exit();
        }
    }

    fn suspended(&mut self,
    _event_loop: &ActiveEventLoop) {
        self.context = None;
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
            WindowEvent::Resized(_) | WindowEvent::Occluded(false) => {
                window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                let Some(context) = &mut self.context else {
                    return;
                };
                let size = window.inner_size();
                if size.width == 0 || size.height == 0 {
                    return;
                }
                // No surface texture exists while resize configures the surface.
                if let Err(error) = context.resize(size).and_then(|()| context.render(window)) {
                    error!(%error, "Rendering stopped");
                    self.failure = Some(error.to_string());
                    event_loop.exit();
                }
            }
            _ => {}
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
