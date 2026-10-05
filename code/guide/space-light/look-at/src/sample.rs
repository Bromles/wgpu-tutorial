use encase::UniformBuffer;
use glam::{Mat4, Vec3};
use framework::{Gpu, Sample};
use std::error::Error;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBinding, BufferBindingType, BufferDescriptor,
    BufferSize, BufferUsages, Color, ColorTargetState, ColorWrites, CommandEncoder, FragmentState,
    FrontFace, LoadOp, Operations, PipelineCompilationOptions, PipelineLayoutDescriptor,
    PrimitiveState, PrimitiveTopology, RenderPassColorAttachment, RenderPassDescriptor,
    RenderPipeline, RenderPipelineDescriptor, ShaderStages, StoreOp, TextureView, VertexAttribute,
    VertexBufferLayout, VertexFormat, VertexState, VertexStepMode, include_wgsl,
 MultisampleState, BindingResource,};

use crate::camera::manual_view;


use bytemuck::Pod;
use bytemuck::Zeroable;
use tracing::info;
use std::f32::consts::FRAC_PI_2;
use bytemuck::cast_slice;
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    position: [f32; 4],
    color: [f32; 4],
}

// Control frame: the same asymmetric triangle as in chapter 16.
const VERTICES: [Vertex; 3] = [
    Vertex {
        position: [-0.25, -0.2, 0.5, 1.0],
        color: [1.0, 0.0, 0.0, 1.0],
    },
    Vertex {
        position: [0.35, -0.15, 0.5, 1.0],
        color: [0.0, 1.0, 0.0, 1.0],
    },
    Vertex {
        position: [0.0, 0.3, 0.5, 1.0],
        color: [0.0, 0.0, 1.0, 1.0],
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

/// Chapter 18: chapter 17's control frame; the view experiment is numerical, in camera.rs.
pub struct LookAt {
    pipeline: RenderPipeline,
    bind_group: BindGroup,
    vertex_buffer: Buffer,
}

impl Sample for LookAt {
    fn init(gpu: &Gpu) -> Result<Self, Box<dyn Error>> {
        let eye = Vec3::new(0.0, 0.0, 5.0);
        let view = manual_view(eye, Vec3::ZERO, Vec3::Y)?;
        info!(?view, "Chapter view matrix");
        let shader = gpu
            .device
            .create_shader_module(include_wgsl!("shader.wgsl"));
        let layout = gpu
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Control layout"),
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
                label: Some("Control pipeline layout"),
                bind_group_layouts: &[Some(&layout)],
                immediate_size: 0,
            });
        let pipeline = gpu
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Look-at control pipeline"),
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
        let transform_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Control transform"),
            size: 64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let transform = Mat4::from_translation(Vec3::new(0.25, 0.0, 0.0))
            * Mat4::from_rotation_z(FRAC_PI_2);
        let mut bytes = UniformBuffer::new(Vec::<u8>::new());
        bytes.write(&transform).expect("fits the uniform contract");
        gpu.queue
            .write_buffer(&transform_buffer, 0, &bytes.into_inner());
        let bind_group = gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Control bind group"),
            layout: &layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: &transform_buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });
        let vertex_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Triangle vertices"),
            size: size_of_val(&VERTICES) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&vertex_buffer, 0, cast_slice(&VERTICES));
        Ok(Self {
            pipeline,
            bind_group,
            vertex_buffer,
        })
    }

    fn draw(&mut self,
    _gpu: &Gpu,
    encoder: &mut CommandEncoder,
    view: &TextureView) {
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Look-at control pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view,
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
            ..RenderPassDescriptor::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.draw(0..3, 0..1);
    }
}
