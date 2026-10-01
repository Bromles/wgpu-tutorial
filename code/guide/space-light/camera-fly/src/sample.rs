use std::collections::HashSet;
use std::error::Error;
use std::time::Instant;

use encase::UniformBuffer;
use shell::{Gpu, Sample};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBinding, BufferBindingType, BufferDescriptor,
    BufferSize, BufferUsages, Color, ColorTargetState, ColorWrites, CommandEncoder,
    CompareFunction, DepthStencilState, Extent3d, FragmentState, FrontFace, LoadOp, Operations,
    PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology,
    RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor,
    RenderPipeline, RenderPipelineDescriptor, ShaderStages, StoreOp, Texture, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages, TextureView, TextureViewDescriptor,
    VertexAttribute, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode, include_wgsl,
};
use winit::dpi::PhysicalSize;
use winit::event::{DeviceEvent, ElementState, MouseButton, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Window};

use crate::camera::{Camera, START_PITCH, START_POSITION, START_YAW};
use glam::Mat4;

/// Frame delta cap; above it the process was paused and the time is dropped.
const MAX_DT: f32 = 0.1;

/// Vertical field of view, near and far planes of the perspective projection.
const FOV_Y: f32 = std::f32::consts::FRAC_PI_3;
const NEAR: f32 = 0.1;
const FAR: f32 = 50.0;

/// Matches the default window size until the first `Resized` event.
const FALLBACK_SIZE: (u32, u32) = (800, 600);

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 4],
    color: [f32; 4],
}

const FLOOR_COLOR: [f32; 4] = [0.42, 0.45, 0.5, 1.0];
const TRIANGLE_A_COLOR: [f32; 4] = [0.85, 0.25, 0.25, 1.0];
const TRIANGLE_B_COLOR: [f32; 4] = [0.25, 0.4, 0.85, 1.0];

const VERTICES: [Vertex; 12] = [
    // Floor: two triangles covering y = 0 over a 12x12 meter square.
    Vertex {
        position: [-6.0, 0.0, -6.0, 1.0],
        color: FLOOR_COLOR,
    },
    Vertex {
        position: [6.0, 0.0, -6.0, 1.0],
        color: FLOOR_COLOR,
    },
    Vertex {
        position: [6.0, 0.0, 6.0, 1.0],
        color: FLOOR_COLOR,
    },
    Vertex {
        position: [-6.0, 0.0, -6.0, 1.0],
        color: FLOOR_COLOR,
    },
    Vertex {
        position: [6.0, 0.0, 6.0, 1.0],
        color: FLOOR_COLOR,
    },
    Vertex {
        position: [-6.0, 0.0, 6.0, 1.0],
        color: FLOOR_COLOR,
    },
    // The crossing triangles of chapter 20; the depth buffer picks the nearest.
    Vertex {
        position: [-1.7, 0.1, 0.0, 1.0],
        color: TRIANGLE_A_COLOR,
    },
    Vertex {
        position: [1.7, 0.1, 0.0, 1.0],
        color: TRIANGLE_A_COLOR,
    },
    Vertex {
        position: [0.0, 2.6, 0.0, 1.0],
        color: TRIANGLE_A_COLOR,
    },
    Vertex {
        position: [0.0, 0.4, -1.7, 1.0],
        color: TRIANGLE_B_COLOR,
    },
    Vertex {
        position: [0.0, 0.4, 1.7, 1.0],
        color: TRIANGLE_B_COLOR,
    },
    Vertex {
        position: [0.0, 2.8, 0.0, 1.0],
        color: TRIANGLE_B_COLOR,
    },
];

impl Vertex {
    const LAYOUT: VertexBufferLayout<'static> = VertexBufferLayout {
        array_stride: 32,
        step_mode: VertexStepMode::Vertex,
        attributes: &[
            VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: 0,
                shader_location: 0,
            },
            VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: 16,
                shader_location: 1,
            },
        ],
    };
}

/// Chapter 21: chapter 20 scene plus a floor, steered by keyboard and mouse.
pub struct CameraFly {
    pipeline: RenderPipeline,
    bind_group: BindGroup,
    view_proj_buffer: Buffer,
    vertex_buffer: Buffer,
    camera: Camera,
    projection: Mat4,
    depth_texture: Option<Texture>,
    depth_view: Option<TextureView>,
    depth_size: (u32, u32),
    /// Nonzero target size from the resize contract; consumed by the next draw.
    pending_size: Option<PhysicalSize<u32>>,
    pressed: HashSet<KeyCode>,
    cursor_locked: bool,
    last_instant: Option<Instant>,
}

impl Sample for CameraFly {
    fn init(gpu: &Gpu) -> Result<Self, Box<dyn Error>> {
        let shader = gpu
            .device
            .create_shader_module(include_wgsl!("shader.wgsl"));
        let layout = gpu
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Camera fly layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(64),
                    },
                    count: None,
                }],
            });
        let pipeline_layout = gpu
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Camera fly pipeline layout"),
                bind_group_layouts: &[Some(&layout)],
                immediate_size: 0,
            });
        let pipeline = gpu
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Camera fly pipeline"),
                layout: Some(&pipeline_layout),
                vertex: VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[Some(Vertex::LAYOUT)],
                    compilation_options: PipelineCompilationOptions::default(),
                },
                fragment: Some(FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(ColorTargetState {
                        format: gpu.format,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    })],
                    compilation_options: PipelineCompilationOptions::default(),
                }),
                primitive: PrimitiveState {
                    topology: PrimitiveTopology::TriangleList,
                    front_face: FrontFace::Ccw,
                    cull_mode: None,
                    ..PrimitiveState::default()
                },
                depth_stencil: Some(DepthStencilState {
                    format: TextureFormat::Depth32Float,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(CompareFunction::Less),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                cache: None,
                multiview_mask: None,
            });
        let view_proj_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Camera fly view*projection"),
            size: 64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Camera fly bind group"),
            layout: &layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(BufferBinding {
                    buffer: &view_proj_buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });
        let vertex_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Scene vertices"),
            size: size_of_val(&VERTICES) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&VERTICES));
        Ok(Self {
            pipeline,
            bind_group,
            view_proj_buffer,
            vertex_buffer,
            camera: Camera::new(START_POSITION, START_YAW, START_PITCH),
            projection: glam::camera::rh::proj::directx::perspective(
                FOV_Y,
                FALLBACK_SIZE.0 as f32 / FALLBACK_SIZE.1 as f32,
                NEAR,
                FAR,
            ),
            depth_texture: None,
            depth_view: None,
            depth_size: (0, 0),
            pending_size: None,
            pressed: HashSet::new(),
            cursor_locked: false,
            last_instant: None,
        })
    }

    /// Called once after init and on every resize; the offscreen harness calls it before its draw.
    fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.pending_size = Some(PhysicalSize::new(width, height));
        }
    }

    fn draw(&mut self, gpu: &Gpu, encoder: &mut CommandEncoder, view: &TextureView) {
        let now = Instant::now();
        // Clamp the raw delta: a pause must not become one huge teleporting step.
        let dt = self
            .last_instant
            .map_or(0.0, |last| now.duration_since(last).as_secs_f32())
            .min(MAX_DT);
        self.last_instant = Some(now);

        // One update per frame consumes the accumulated input state.
        self.camera.update(dt, &self.pressed);

        if let Some(size) = self.pending_size.take()
            && self.depth_size != (size.width, size.height)
        {
            self.recreate_depth(gpu, size.width, size.height);
        }

        let view_proj = self.projection * self.camera.view_matrix();
        let mut bytes = UniformBuffer::new(Vec::<u8>::new());
        bytes.write(&view_proj).expect("fits the uniform contract");
        gpu.queue
            .write_buffer(&self.view_proj_buffer, 0, &bytes.into_inner());

        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Camera fly pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color {
                        r: 0.35,
                        g: 0.4,
                        b: 0.5,
                        a: 1.0,
                    }),
                    store: StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: self
                    .depth_view
                    .as_ref()
                    .expect("depth exists after recreate"),
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(1.0),
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..RenderPassDescriptor::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.draw(0..VERTICES.len() as u32, 0..1);
    }

    fn window_event(&mut self, window: &Window, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput {
                event: key_event, ..
            } => {
                if let PhysicalKey::Code(code) = key_event.physical_key {
                    match key_event.state {
                        ElementState::Pressed => {
                            // insert() is false while held: OS key repeat must not retrigger one-shots.
                            if self.pressed.insert(code) {
                                match code {
                                    KeyCode::KeyR => self.camera.reset(),
                                    KeyCode::AltLeft | KeyCode::AltRight => {
                                        self.release_cursor(window)
                                    }
                                    _ => {}
                                }
                            }
                        }
                        ElementState::Released => {
                            self.pressed.remove(&code);
                        }
                    }
                }
            }
            WindowEvent::CursorMoved { .. } => {
                // Locked-cursor positions get clamped/frozen by the OS; motion arrives as a device event.
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                if !self.cursor_locked {
                    self.grab_cursor(window);
                }
            }
            WindowEvent::Focused(false) => {
                self.release_cursor(window);
                self.pressed.clear();
            }
            _ => {}
        }
        if matches!(event, WindowEvent::RedrawRequested) {
            window.request_redraw();
        }
    }

    fn device_event(&mut self, event: &DeviceEvent) {
        // Relative mouse motion: keeps flowing while locked, skips the OS pointer curve.
        if self.cursor_locked
            && let DeviceEvent::MouseMotion { delta } = event
        {
            self.camera.add_mouse_delta(delta.0 as f32, delta.1 as f32);
        }
    }
}

impl CameraFly {
    /// Recreates the depth attachment and projection for a new target size.
    fn recreate_depth(&mut self, gpu: &Gpu, width: u32, height: u32) {
        let texture = gpu.device.create_texture(&TextureDescriptor {
            label: Some("Camera fly depth"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&TextureViewDescriptor::default());
        self.depth_texture = Some(texture);
        self.depth_view = Some(view);
        self.depth_size = (width, height);
        let aspect = width as f32 / height as f32;
        self.projection =
            glam::camera::rh::proj::directx::perspective(FOV_Y, aspect, NEAR, FAR);
    }

    /// Locks the cursor on the first click; an unavailable lock is reported, not ignored.
    fn grab_cursor(&mut self, window: &Window) {
        match window.set_cursor_grab(CursorGrabMode::Locked) {
            Ok(()) => {
                window.set_cursor_visible(false);
                self.cursor_locked = true;
            }
            Err(error) => {
                tracing::warn!(%error, "cursor lock unavailable; steering stays on the keyboard");
            }
        }
    }

    /// Releases on Alt or focus loss; keys and pending delta die with the focus.
    fn release_cursor(&mut self, window: &Window) {
        if let Err(error) = window.set_cursor_grab(CursorGrabMode::None) {
            tracing::warn!(%error, "releasing the cursor failed");
        }
        window.set_cursor_visible(true);
        self.cursor_locked = false;
        self.camera.clear_pending_delta();
    }
}
