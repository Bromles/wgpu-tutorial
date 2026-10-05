use std::{error::Error, sync::Arc};

use winit::{dpi::PhysicalSize, window::Window};

use wgpu::{Backends, Instance, PresentMode, SurfaceColorSpace, TextureFormat, TextureUsages};
use crate::{gpu::Gpu, sample::Sample, settings::Settings};
use std::sync::Mutex;
use wgpu::Surface;
use wgpu::Device;
use wgpu::Queue;
use wgpu::SurfaceConfiguration;
use wgpu::InstanceDescriptor;
use pollster::block_on;
use wgpu::RequestAdapterOptions;
use tracing::error;
use tracing::info;
use wgpu::SurfaceTexture;
use wgpu::TextureViewDescriptor;
use wgpu::CommandEncoderDescriptor;

/// Terminal failure shared with the uncaptured-error handler, which runs on
/// an arbitrary thread and cannot reach `App` directly.
pub(crate) type SharedFailure = Arc<Mutex<Option<String>>>;

pub(crate) struct Context {
    pub(crate) surface: Surface<'static>,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    pub(crate) needs_configure: bool,
}

impl Context {
    pub(crate) fn new(
        window: Arc<Window>,
        settings: &Settings,
        shared_failure: &SharedFailure,
    ) -> Result<Self, Box<dyn Error>> {
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
            block_on(adapter.request_device(&settings.device_descriptor))?;
        // Record instead of exiting: the event loop keeps running until the
        // next redraw notices the failure and unwinds normally.
        let failure_sink = shared_failure.clone();
        device.on_uncaptured_error(Arc::new(move |error| {
            error!(%error, "Unrecoverable GPU error");
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
            needs_configure: true,
        })
    }

    pub(crate) fn gpu(&self) -> Gpu {
        Gpu {
            device: self.device.clone(),
            queue: self.queue.clone(),
            format: self.config.format,
        }
    }

    pub(crate) fn resize(&mut self,
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

    pub(crate) fn render<S: Sample>(
        &mut self,
        window: &Window,
        frame: SurfaceTexture,
        sample: &mut S,
    ) {
        let view = frame.texture.create_view(&TextureViewDescriptor {
            label: Some("Framework surface view"),
            ..Default::default()
        });
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Framework encoder"),
            });
        sample.draw(&self.gpu(), &mut encoder, &view);
        self.queue.submit([encoder.finish()]);
        window.pre_present_notify();
        self.queue.present(frame);
        info!(
            width = self.config.width,
            height = self.config.height,
            "Rendered frame"
        );
    }
}
