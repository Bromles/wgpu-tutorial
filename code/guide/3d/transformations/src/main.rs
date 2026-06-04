#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::f32::consts::FRAC_PI_4;
use std::mem::size_of;
use std::time::Instant;

use bytemuck::{Pod, Zeroable};
use encase::ShaderType;
use glam::Mat4;
use wgpu::util::DeviceExt;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, BlendComponent, BlendState, Buffer, BufferAddress,
    BufferBindingType, BufferDescriptor, BufferUsages, Color, ColorTargetState, ColorWrites,
    CommandEncoder, FragmentState, IndexFormat, LoadOp, MultisampleState, Operations,
    PipelineCompilationOptions, PipelineLayoutDescriptor, PolygonMode, PrimitiveState,
    PrimitiveTopology, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, ShaderStages, StoreOp, TextureView, VertexAttribute,
    VertexBufferLayout, VertexFormat, VertexState, VertexStepMode, include_wgsl,
};

use framework::{Example, GpuContext, run};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

impl Vertex {
    const ATTRIBUTES: [VertexAttribute; 2] = [
        VertexAttribute {
            offset: 0,
            shader_location: 0,
            format: VertexFormat::Float32x3,
        },
        VertexAttribute {
            offset: size_of::<[f32; 3]>() as BufferAddress,
            shader_location: 1,
            format: VertexFormat::Float32x3,
        },
    ];

    fn desc() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: size_of::<Vertex>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

const VERTICES: &[Vertex] = &[
    Vertex {
        position: [-0.5, -0.5, 0.5],
        color: [1.0, 0.0, 0.0],
    },
    Vertex {
        position: [0.5, -0.5, 0.5],
        color: [0.0, 1.0, 0.0],
    },
    Vertex {
        position: [0.5, 0.5, 0.5],
        color: [0.0, 0.0, 1.0],
    },
    Vertex {
        position: [-0.5, 0.5, 0.5],
        color: [1.0, 1.0, 0.0],
    },
    Vertex {
        position: [-0.5, -0.5, -0.5],
        color: [1.0, 0.0, 1.0],
    },
    Vertex {
        position: [0.5, -0.5, -0.5],
        color: [0.0, 1.0, 1.0],
    },
    Vertex {
        position: [0.5, 0.5, -0.5],
        color: [0.5, 0.5, 0.5],
    },
    Vertex {
        position: [-0.5, 0.5, -0.5],
        color: [1.0, 0.5, 0.0],
    },
];

const INDICES: &[u16] = &[
    0, 1, 2, 2, 3, 0, 1, 5, 6, 6, 2, 1, 5, 4, 7, 7, 6, 5, 4, 0, 3, 3, 7, 4, 3, 2, 6, 6, 7, 3, 4, 5,
    1, 1, 0, 4,
];

#[derive(ShaderType)]
struct ShaderUniforms {
    mvp: Mat4,
}

struct RotatingCube {
    pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    uniform_buffer: Buffer,
    bind_group: BindGroup,
    start_time: Instant,
}

impl Example for RotatingCube {
    fn init(ctx: &GpuContext) -> Self {
        let shader_module = ctx
            .device
            .create_shader_module(include_wgsl!("shader.wgsl"));

        let vertex_buffer = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(VERTICES),
                usage: BufferUsages::VERTEX,
            });

        let index_buffer = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(INDICES),
                usage: BufferUsages::INDEX,
            });

        let uniform_buffer = ctx.device.create_buffer(&BufferDescriptor {
            label: Some("Uniform Buffer"),
            size: ShaderUniforms::min_size().into(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = ctx
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Bind Group Layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(ShaderUniforms::min_size()),
                    },
                    count: None,
                }],
            });

        let bind_group = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Bind Group"),
            layout: &bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = ctx
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Pipeline Layout"),
                bind_group_layouts: &[Some(&bind_group_layout)],
                immediate_size: 0,
            });

        let pipeline = ctx
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Render Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: VertexState {
                    module: &shader_module,
                    entry_point: Some("vs_main"),
                    buffers: &[Vertex::desc()],
                    compilation_options: PipelineCompilationOptions::default(),
                },
                fragment: Some(FragmentState {
                    module: &shader_module,
                    entry_point: Some("fs_main"),
                    targets: &[Some(ColorTargetState {
                        format: ctx.surface_format,
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
                    front_face: wgpu::FrontFace::Ccw,
                    polygon_mode: PolygonMode::Fill,
                    cull_mode: Some(wgpu::Face::Back),
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                cache: None,
                multiview_mask: None,
            });

        Self {
            pipeline,
            vertex_buffer,
            index_buffer,
            uniform_buffer,
            bind_group,
            start_time: Instant::now(),
        }
    }

    fn render(&mut self, ctx: &GpuContext, view: &TextureView, encoder: &mut CommandEncoder) {
        let time = self.start_time.elapsed().as_secs_f32();

        let aspect = ctx.surface_config.width as f32 / ctx.surface_config.height as f32;
        let projection = Mat4::perspective_rh(FRAC_PI_4, aspect, 0.1, 100.0);
        let view_mat = Mat4::look_at_rh(
            glam::Vec3::new(2.0, 1.5, 2.0),
            glam::Vec3::ZERO,
            glam::Vec3::Y,
        );
        let model = Mat4::from_rotation_y(time);
        let mvp = projection * view_mat * model;

        let mut uniform_data = encase::UniformBuffer::new(Vec::new());
        uniform_data.write(&ShaderUniforms { mvp }).unwrap();
        ctx.queue
            .write_buffer(&self.uniform_buffer, 0, &uniform_data.into_inner());

        let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::BLACK),
                    store: StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        rpass.set_pipeline(&self.pipeline);
        rpass.set_bind_group(0, &self.bind_group, &[]);
        rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        rpass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint16);
        rpass.draw_indexed(0..36, 0, 0..1);
    }
}

fn main() {
    run::<RotatingCube>("MVP Transformations");
}
