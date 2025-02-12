#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;

use tokio::runtime;
use tokio::runtime::Runtime;
use wgpu::{
    Backends, BlendComponent, BlendState, Color, ColorTargetState, ColorWrites, CommandEncoderDescriptor,
    CompositeAlphaMode, Device, DeviceDescriptor, Features, FragmentState,
    FrontFace, include_wgsl, Instance, InstanceDescriptor, Limits, LoadOp, MemoryHints,
    MultisampleState, Operations, PipelineCompilationOptions, PolygonMode, PowerPreference,
    PresentMode, PrimitiveState, PrimitiveTopology, Queue, RenderPassColorAttachment,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, RequestAdapterOptions, StoreOp,
    Surface, SurfaceConfiguration, SurfaceError, TextureFormat, TextureUsages,
    TextureViewDescriptor, VertexState,
};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowAttributes, WindowId};

enum App {
    Loading,
    Ready {
        window: Arc<Window>,
        renderer: Renderer,
        need_to_resize_surface: bool,
    },
}

struct Renderer {
    device: Device,
    queue: Queue,
    surface: Surface<'static>,
    surface_config: SurfaceConfiguration,
    surface_format: TextureFormat,
    pipeline: Option<RenderPipeline>,
}

impl Renderer {
    fn new(window: Arc<Window>, runtime: Arc<Runtime>) -> Self {
        let mut physical_size = window.inner_size();
        physical_size.width = physical_size.width.max(1);
        physical_size.height = physical_size.height.max(1);

        let instance = Instance::new(&InstanceDescriptor {
            backends: Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance
            .create_surface(window)
            .expect("Failed to create surface");
        let adapter = runtime.block_on(async {
            instance
                .request_adapter(&RequestAdapterOptions {
                    power_preference: PowerPreference::default(),
                    force_fallback_adapter: false,
                    compatible_surface: Some(&surface),
                })
                .await
                .expect("Failed to request adapter")
        });

        let (device, queue) = runtime.block_on(async {
            adapter
                .request_device(
                    &DeviceDescriptor {
                        label: None,
                        required_features: adapter.features() & Features::default(),
                        required_limits: Limits::default().using_resolution(adapter.limits()),
                        memory_hints: MemoryHints::Performance,
                    },
                    None,
                )
                .await
                .expect("Failed to request device")
        });

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
            present_mode: PresentMode::AutoNoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: CompositeAlphaMode::Auto,
            view_formats: vec![],
        };

        let mut renderer = Self {
            device,
            queue,
            surface,
            surface_config,
            surface_format,
            pipeline: None,
        };

        renderer.resize_surface(physical_size);

        let shader_module = renderer
            .device
            .create_shader_module(include_wgsl!("shader.wgsl"));

        renderer.pipeline = Some(renderer.device.create_render_pipeline(
            &RenderPipelineDescriptor {
                label: None,
                layout: None,
                vertex: VertexState {
                    module: &shader_module,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: PipelineCompilationOptions::default(),
                },
                fragment: Some(FragmentState {
                    module: &shader_module,
                    entry_point: Some("fs_main"),
                    targets: &[Some(ColorTargetState {
                        format: renderer.surface_format,
                        blend: Some(BlendState {
                            color: BlendComponent::REPLACE,
                            alpha: BlendComponent::REPLACE,
                        }),
                        write_mask: ColorWrites::ALL,
                    })],
                    compilation_options: PipelineCompilationOptions::default(),
                }),
                primitive: PrimitiveState {
                    topology: PrimitiveTopology::TriangleList,
                    front_face: FrontFace::Ccw,
                    polygon_mode: PolygonMode::Fill,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
                cache: None,
            },
        ));

        renderer
    }

    fn resize_surface(&self, size: PhysicalSize<u32>) {
        let width = size.width.max(1);
        let height = size.height.max(1);

        self.surface.configure(
            &self.device,
            &SurfaceConfiguration {
                width,
                height,
                ..self.surface_config.clone()
            },
        );
    }

    fn render(&mut self, window: Arc<Window>) {
        match self.surface.get_current_texture() {
            Ok(frame) => {
                let mut encoder = self
                    .device
                    .create_command_encoder(&CommandEncoderDescriptor { label: None });

                let view = frame.texture.create_view(&TextureViewDescriptor::default());

                {
                    let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                        label: None,
                        color_attachments: &[Some(RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: Operations {
                                load: LoadOp::Clear(Color::GREEN),
                                store: StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });

                    rpass.set_pipeline(self.pipeline.as_ref().expect("Failed to get pipeline"));
                    rpass.draw(0..3, 0..1);
                }

                self.queue.submit([encoder.finish()]);
                window.pre_present_notify();
                frame.present();
            }
            Err(error) => match error {
                SurfaceError::OutOfMemory => {
                    panic!("Surface error: {error}")
                }
                _ => {
                    window.request_redraw();
                }
            },
        };
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Self::Loading = self {
            let runtime = Arc::new(
                runtime::Builder::new_current_thread()
                    .build()
                    .expect("Failed to create tokio runtime"),
            );

            let window_attributes = WindowAttributes::default()
                .with_title("WGPU Tutorial")
                .with_visible(false);

            let window = Arc::new(
                event_loop
                    .create_window(window_attributes)
                    .expect("Failed to create window"),
            );

            center_window(window.clone());

            event_loop.set_control_flow(ControlFlow::Wait);

            let renderer = Renderer::new(window.clone(), runtime.clone());

            *self = Self::Ready {
                window,
                renderer,
                need_to_resize_surface: false,
            }
        }

        let Self::Ready {
            window, renderer, ..
        } = self
        else {
            return;
        };

        renderer.render(window.clone());

        window.set_visible(true);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let Self::Ready {
            window,
            renderer,
            need_to_resize_surface,
            ..
        } = self
        else {
            return;
        };

        match event {
            WindowEvent::RedrawRequested => {
                if *need_to_resize_surface {
                    let size = window.inner_size();

                    renderer.resize_surface(size);

                    *need_to_resize_surface = false;
                }

                renderer.render(window.clone());

                window.request_redraw();
            }
            WindowEvent::Resized(_) => {
                *need_to_resize_surface = true;
                window.request_redraw();
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::KeyboardInput { event, .. } => handle_keyboard_input(event_loop, event),
            _ => {}
        }
    }
}

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let event_loop = EventLoop::new().expect("Failed to create event loop");

    let mut app = App::Loading;

    event_loop
        .run_app(&mut app)
        .expect("Failed to run event loop");
}

fn handle_keyboard_input(event_loop: &ActiveEventLoop, event: KeyEvent) {
    match (event.physical_key, event.state) {
        (PhysicalKey::Code(KeyCode::Escape), ElementState::Pressed) => {
            event_loop.exit();
        }
        _ => {}
    }
}

fn center_window(window: Arc<Window>) {
    if let Some(monitor) = window.current_monitor() {
        let screen_size = monitor.size();
        let window_size = window.outer_size();

        window.set_outer_position(winit::dpi::PhysicalPosition {
            x: screen_size.width.saturating_sub(window_size.width) as f64 / 2.0
                + monitor.position().x as f64,
            y: screen_size.height.saturating_sub(window_size.height) as f64 / 2.0
                + monitor.position().y as f64,
        });
    }
}
