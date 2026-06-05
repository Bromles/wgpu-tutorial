use std::sync::Arc;

use tracing::warn;
use wgpu::CurrentSurfaceTexture::{
    Lost, Occluded, Outdated, Suboptimal, Success, Timeout, Validation,
};
use wgpu::*;
use winit::dpi::PhysicalSize;
use winit::window::Window;

pub struct GpuContext {
    pub device: Device,
    pub queue: Queue,
    pub surface: Surface<'static>,
    pub surface_config: SurfaceConfiguration,
    pub surface_format: TextureFormat,
}

impl GpuContext {
    pub fn new(window: Arc<Window>) -> Self {
        let mut physical_size = window.inner_size();
        physical_size.width = physical_size.width.max(1);
        physical_size.height = physical_size.height.max(1);

        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::PRIMARY,
            ..InstanceDescriptor::new_without_display_handle()
        });

        let surface = instance
            .create_surface(window)
            .expect("Failed to create surface");

        let adapter = pollster::block_on(instance.request_adapter(&RequestAdapterOptions {
            power_preference: PowerPreference::default(),
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        }))
        .expect("Failed to request adapter");

        let (device, queue) = pollster::block_on(adapter.request_device(&DeviceDescriptor {
            label: Some("Main device"),
            required_features: adapter.features()
                - Features::EXPERIMENTAL_RAY_QUERY
                - Features::EXPERIMENTAL_MESH_SHADER
                - Features::EXPERIMENTAL_COOPERATIVE_MATRIX,
            required_limits: Limits::default().using_resolution(adapter.limits()),
            memory_hints: MemoryHints::Performance,
            trace: Default::default(),
            experimental_features: ExperimentalFeatures::disabled(),
        }))
        .expect("Failed to request device");

        let surface_capabilities = surface.get_capabilities(&adapter);

        let surface_format = surface_capabilities
            .formats
            .iter()
            .copied()
            .find(TextureFormat::is_srgb)
            .or_else(|| surface_capabilities.formats.first().copied())
            .expect("Failed to get surface format");

        let surface_config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: physical_size.width,
            height: physical_size.height,
            present_mode: PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: CompositeAlphaMode::Auto,
            view_formats: vec![],
        };

        surface.configure(&device, &surface_config);

        Self {
            device,
            queue,
            surface,
            surface_config,
            surface_format,
        }
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.surface_config.width = size.width.max(1);
        self.surface_config.height = size.height.max(1);
        self.surface.configure(&self.device, &self.surface_config);
    }

    pub fn acquire_frame(&mut self) -> Option<(SurfaceTexture, TextureView, CommandEncoder)> {
        let frame = match self.surface.get_current_texture() {
            Success(frame) => frame,
            Suboptimal(frame) => {
                warn!("Surface suboptimal, reconfiguring");
                self.surface.configure(&self.device, &self.surface_config);
                frame
            }
            Outdated | Lost => {
                warn!("Surface lost or outdated, reconfiguring");
                self.surface.configure(&self.device, &self.surface_config);
                return None;
            }
            Timeout | Occluded => return None,
            Validation => {
                warn!("Surface texture validation error");
                return None;
            }
        };

        let view = frame.texture.create_view(&TextureViewDescriptor::default());

        let encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Main command encoder"),
            });

        Some((frame, view, encoder))
    }
}
