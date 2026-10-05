use bytemuck::Pod;
use bytemuck::Zeroable;
use bytemuck::cast_slice;
use crate::params::Params;
use encase::UniformBuffer;
use framework::{Gpu, Sample};
use glam::camera::rh::proj::directx::perspective;
use glam::camera::rh::view::look_at_mat4;
use glam::{Mat4, Vec3};
use std::error::Error;
use std::f32::consts::FRAC_PI_3;
use wgpu::BindingResource;
use wgpu::IndexFormat;
use wgpu::MultisampleState;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBinding, BufferBindingType, BufferDescriptor,
    BufferSize, BufferUsages, Color, ColorTargetState, ColorWrites, CommandEncoder, FragmentState,
    FrontFace, LoadOp, Operations, PipelineCompilationOptions, PipelineLayoutDescriptor,
    PrimitiveState, PrimitiveTopology, RenderPassColorAttachment, RenderPassDescriptor,
    RenderPipeline, RenderPipelineDescriptor, ShaderStages, StoreOp, TextureView, VertexAttribute,
    VertexBufferLayout, VertexFormat, VertexState, VertexStepMode, include_wgsl,
};
use winit::event::{ElementState, WindowEvent};
use winit::keyboard::KeyCode;
use winit::keyboard::PhysicalKey;
use winit::window::Window;

/// Fixed camera from chapter 25: three metres in front, looking at the origin.
const EYE: Vec3 = Vec3::new(0.0, 0.0, 3.0);
const FOV_Y: f32 = FRAC_PI_3;
const NEAR: f32 = 0.1;
const FAR: f32 = 50.0;

/// Matches the default window size until the first `Resized` event.
const FALLBACK_SIZE: (u32, u32) = (800, 600);

/// Arrow-key step for the source position, metres.
const MOVE_STEP: f32 = 0.25;
/// The source never sinks onto the plane: at r = 0 the light direction is undefined.
const MIN_HEIGHT: f32 = 0.25;
const MAX_HEIGHT: f32 = 4.0;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    /// Position with w = 1: a point of the z = 0 plane.
    position: [f32; 4],
    /// Normal with w = 0: the plane normal +Z for every corner.
    normal: [f32; 4],
}

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

/// Plane half extents, as in chapter 25.
const HALF_X: f32 = 2.0;
const HALF_Y: f32 = 1.5;

fn plane_vertices() -> [Vertex; 4] {
    let normal = [0.0, 0.0, 1.0, 0.0];
    [
        Vertex {
            position: [-HALF_X, -HALF_Y, 0.0, 1.0],
            normal,
        },
        Vertex {
            position: [HALF_X, -HALF_Y, 0.0, 1.0],
            normal,
        },
        Vertex {
            position: [-HALF_X, HALF_Y, 0.0, 1.0],
            normal,
        },
        Vertex {
            position: [HALF_X, HALF_Y, 0.0, 1.0],
            normal,
        },
    ]
}

// Two triangles over the four corners; the shared diagonal keeps the plane flat.
const INDICES: [u16; 6] = [0, 1, 2, 2, 1, 3];

/// Chapter 26a: chapter 25's plane and camera with a positional source; S toggles the cone, arrows move it.
pub struct LightPoint {
    pipeline: RenderPipeline,
    bind_group: BindGroup,
    params_buffer: Buffer,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    light_pos: Vec3,
    spot_on: bool,
    projection: Mat4,
}

impl Sample for LightPoint {
    fn init(gpu: &Gpu) -> Result<Self, Box<dyn Error>> {
        let vertices = plane_vertices();
        let shader = gpu
            .device
            .create_shader_module(include_wgsl!("shader.wgsl"));
        let layout = gpu
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Point light layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(112),
                    },
                    count: None,
                }],
            });
        let pipeline_layout = gpu
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Point light pipeline layout"),
                bind_group_layouts: &[Some(&layout)],
                immediate_size: 0,
            });
        let pipeline = gpu
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Point light pipeline"),
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
                depth_stencil: None,
                multisample: MultisampleState::default(),
                cache: None,
                multiview_mask: None,
            });
        let params_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Point light params"),
            size: 112,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Point light bind group"),
            layout: &layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: &params_buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });
        let vertex_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Plane vertices"),
            size: size_of_val(&vertices) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&vertex_buffer, 0, cast_slice(&vertices));
        let index_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Plane indices"),
            size: size_of_val(&INDICES) as u64,
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&index_buffer, 0, cast_slice(&INDICES));
        Ok(Self {
            pipeline,
            bind_group,
            params_buffer,
            vertex_buffer,
            index_buffer,
            light_pos: Vec3::new(0.0, 0.0, 2.0),
            spot_on: false,
            projection: perspective(
                FOV_Y,
                FALLBACK_SIZE.0 as f32 / FALLBACK_SIZE.1 as f32,
                NEAR,
                FAR,
            ),
        })
    }

    /// Called once after init and on every resize; the offscreen harness calls it before its draw.
    fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.projection = perspective(FOV_Y, width as f32 / height as f32, NEAR, FAR);
        }
    }

    fn draw(&mut self, gpu: &Gpu, encoder: &mut CommandEncoder, view: &TextureView) {
        let view_proj = self.projection * look_at_mat4(EYE, Vec3::ZERO, Vec3::Y);
        let mut params = Params::new(view_proj);
        params.light_pos = self.light_pos;
        params.spot_on = u32::from(self.spot_on);
        let mut bytes = UniformBuffer::new(Vec::<u8>::new());
        bytes.write(&params).expect("fits the uniform contract");
        gpu.queue
            .write_buffer(&self.params_buffer, 0, &bytes.into_inner());

        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Point light pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color {
                        r: 0.1,
                        g: 0.1,
                        b: 0.12,
                        a: 1.0,
                    }),
                    store: StoreOp::Store,
                },
                depth_slice: None,
            })],
            ..RenderPassDescriptor::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint16);
        pass.draw_indexed(0..INDICES.len() as u32, 0, 0..1);
    }

    fn window_event(&mut self, window: &Window, event: &WindowEvent) {
        if let WindowEvent::KeyboardInput {
            event: key_event, ..
        } = event
            && key_event.state == ElementState::Pressed
            && let PhysicalKey::Code(key_code) = key_event.physical_key
        {
            match key_code {
                KeyCode::KeyS => {
                    self.spot_on = !self.spot_on;
                    window.request_redraw();
                }
                KeyCode::ArrowLeft => {
                    self.light_pos.x -= MOVE_STEP;
                    window.request_redraw();
                }
                KeyCode::ArrowRight => {
                    self.light_pos.x += MOVE_STEP;
                    window.request_redraw();
                }
                KeyCode::ArrowUp => {
                    self.light_pos.z += MOVE_STEP;
                    window.request_redraw();
                }
                KeyCode::ArrowDown => {
                    self.light_pos.z -= MOVE_STEP;
                    window.request_redraw();
                }
                _ => {}
            }
            // The height keeps a positive margin: the source may approach but never land.
            self.light_pos.z = self.light_pos.z.clamp(MIN_HEIGHT, MAX_HEIGHT);
        }
    }
}

impl LightPoint {
    /// Places the source; driven by the verification crate. The height keeps its margin.
    pub fn set_light_pos(&mut self, pos: Vec3) {
        self.light_pos = Vec3::new(pos.x, 0.0, pos.z.clamp(MIN_HEIGHT, MAX_HEIGHT));
    }

    /// Enables or disables the cone.
    pub fn set_spot_on(&mut self, on: bool) {
        self.spot_on = on;
    }
}
