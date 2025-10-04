#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;

use tokio::runtime;
use tokio::runtime::Runtime;
use wgpu::{
    Backends, Color, CommandEncoderDescriptor, Device, DeviceDescriptor, ExperimentalFeatures,
    Features, Instance, InstanceDescriptor, Limits, LoadOp, MemoryHints, Operations,
    PowerPreference, Queue, RenderPassColorAttachment, RenderPassDescriptor, RequestAdapterOptions,
    StoreOp, Surface, SurfaceConfiguration, SurfaceError, TextureViewDescriptor,
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
        renderer: Box<Renderer>,
        need_to_resize_surface: bool,
    },
}

struct Renderer {
    device: Device,
    queue: Queue,
    surface: Surface<'static>,
    surface_config: SurfaceConfiguration,
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
                .request_device(&DeviceDescriptor {
                    label: Some("Main device"),
                    required_features: adapter.features() & Features::default(),
                    required_limits: Limits::default().using_resolution(adapter.limits()),
                    memory_hints: MemoryHints::Performance,
                    trace: Default::default(),
                    experimental_features: ExperimentalFeatures::disabled(),
                })
                .await
                .expect("Failed to request device")
        });

        let surface_config = surface
            .get_default_config(&adapter, physical_size.width, physical_size.height)
            .expect("Failed to get default surface config");

        surface.configure(&device, &surface_config);

        Self {
            device,
            queue,
            surface,
            surface_config,
        }
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
                    .create_command_encoder(&CommandEncoderDescriptor {
                        label: Some("Main command encoder"),
                    });

                let view = frame.texture.create_view(&TextureViewDescriptor::default());

                encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("Clear render pass"),
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color::GREEN),
                            store: StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

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
                renderer: Box::new(renderer),
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
