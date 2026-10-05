use std::error::Error;

use bytemuck::Pod;
use bytemuck::Zeroable;
use bytemuck::cast_slice;
use encase::UniformBuffer;
use framework::{Gpu, Sample};
use glam::camera::rh::proj::directx::perspective;
use glam::camera::rh::view::look_at_mat4;
use glam::{Mat4, Vec3};
use std::f32::consts::FRAC_1_SQRT_2;
use std::f32::consts::FRAC_PI_3;
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
use winit::window::Window;

use crate::params::ModelParams;
use wgpu::BindingResource;
use wgpu::IndexFormat;
use wgpu::MultisampleState;
use winit::keyboard::KeyCode;
use winit::keyboard::PhysicalKey;

/// Fixed camera: seen along the face normal, at distance 5 on the 45-degree diagonal.
const EYE: Vec3 = Vec3::new(5.0 * FRAC_1_SQRT_2, -5.0 * FRAC_1_SQRT_2, 0.0);
const FOV_Y: f32 = FRAC_PI_3;
const NEAR: f32 = 0.1;
const FAR: f32 = 50.0;

/// Matches the default window size until the first `Resized` event.
const FALLBACK_SIZE: (u32, u32) = (800, 600);

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    /// Position with w = 1: a point.
    position: [f32; 4],
    /// Normal with w = 0: a direction, immune to translation.
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

/// Chapter 23 tangents; the normal is their cross product, so the face comes from one formula.
const TANGENT_T: Vec3 = Vec3::new(1.0, 1.0, 0.0);
const TANGENT_B: Vec3 = Vec3::new(0.0, 0.0, 1.0);

/// Corners: every +-t +-b combination, halved; all carry n = normalize(t x b).
fn face_vertices() -> [Vertex; 4] {
    let normal = TANGENT_T.cross(TANGENT_B).normalize().extend(0.0);
    let corners = [
        (TANGENT_T + TANGENT_B) * 0.5,
        (TANGENT_T - TANGENT_B) * 0.5,
        (-TANGENT_T - TANGENT_B) * 0.5,
        (-TANGENT_T + TANGENT_B) * 0.5,
    ];
    corners.map(|corner| Vertex {
        position: corner.extend(1.0).to_array(),
        normal: normal.to_array(),
    })
}

// Two triangles over the four corners; the shared diagonal keeps the face flat.
const INDICES: [u16; 6] = [0, 1, 2, 0, 2, 3];

/// Chapter 24: chapter 23's face and light, model no longer identity; X toggles an X scale of 1 vs 2.
pub struct NormalMatrix {
    pipeline: RenderPipeline,
    bind_group: BindGroup,
    view_proj_buffer: Buffer,
    model_buffer: Buffer,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    /// Model X scale, toggled between 1 and 2 by the X key.
    scale_x: f32,
    projection: Mat4,
}

impl Sample for NormalMatrix {
    fn init(gpu: &Gpu) -> Result<Self, Box<dyn Error>> {
        let vertices = face_vertices();
        let shader = gpu
            .device
            .create_shader_module(include_wgsl!("shader.wgsl"));
        let layout = gpu
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Normal matrix layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(64),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::VERTEX_FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(128),
                        },
                        count: None,
                    },
                ],
            });
        let pipeline_layout = gpu
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Normal matrix pipeline layout"),
                bind_group_layouts: &[Some(&layout)],
                immediate_size: 0,
            });
        let pipeline = gpu
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Normal matrix pipeline"),
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
        let view_proj_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Normal matrix view*projection"),
            size: 64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let model_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Normal matrix model params"),
            size: 128,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Normal matrix bind group"),
            layout: &layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &view_proj_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &model_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });
        let vertex_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Face vertices"),
            size: size_of_val(&vertices) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&vertex_buffer, 0, cast_slice(&vertices));
        let index_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Face indices"),
            size: size_of_val(&INDICES) as u64,
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&index_buffer, 0, cast_slice(&INDICES));
        Ok(Self {
            pipeline,
            bind_group,
            view_proj_buffer,
            model_buffer,
            vertex_buffer,
            index_buffer,
            scale_x: 1.0,
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
        let mut matrix_bytes = UniformBuffer::new(Vec::<u8>::new());
        matrix_bytes
            .write(&view_proj)
            .expect("fits the uniform contract");
        gpu.queue
            .write_buffer(&self.view_proj_buffer, 0, &matrix_bytes.into_inner());
        let params = ModelParams::from_scale(Vec3::new(self.scale_x, 1.0, 1.0))
            .expect("the key toggles between 1 and 2 only");
        let mut params_bytes = UniformBuffer::new(Vec::<u8>::new());
        params_bytes
            .write(&params)
            .expect("fits the uniform contract");
        gpu.queue
            .write_buffer(&self.model_buffer, 0, &params_bytes.into_inner());

        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Normal matrix pass"),
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
            && let PhysicalKey::Code(KeyCode::KeyX) = key_event.physical_key
        {
            // 1 <-> 2: the only scale pair the chapter compares.
            self.scale_x = if self.scale_x == 1.0 { 2.0 } else { 1.0 };
            window.request_redraw();
        }
    }
}

impl NormalMatrix {
    /// Explicit X scale; driven by the verification crate.
    pub fn set_scale(&mut self, scale_x: f32) {
        self.scale_x = scale_x;
    }
}
