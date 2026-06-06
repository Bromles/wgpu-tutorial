#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::f32::consts::FRAC_PI_4;
use std::mem::size_of;
use std::time::Duration;

use bytemuck::{Pod, Zeroable};
use encase::ShaderType;
use glam::{Mat3, Mat4, Vec3};
use wgpu::util::DeviceExt;
use wgpu::{
    include_wgsl, BindGroup, BindGroupDescriptor, BindGroupEntry,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, BlendComponent, BlendState, Buffer,
    BufferAddress, BufferBindingType, BufferDescriptor, BufferUsages, Color, ColorTargetState,
    ColorWrites, CommandEncoder, CompareFunction, DepthBiasState, DepthStencilState, Extent3d,
    Face, FragmentState, FrontFace, IndexFormat, LoadOp, MultisampleState,
    Operations, PipelineCompilationOptions, PipelineLayoutDescriptor, PolygonMode,
    PrimitiveState, PrimitiveTopology, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, ShaderStages,
    StencilState, StoreOp, Texture, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages, TextureView, TextureViewDescriptor, VertexAttribute, VertexBufferLayout,
    VertexFormat, VertexState, VertexStepMode,
};
use winit::dpi::PhysicalSize;

use framework::{
    run, Camera, Example, GpuContext, Input, CUBE_INDICES, CUBE_NORMALS, CUBE_POSITIONS,
};

const SAMPLE_COUNT: u32 = 4;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
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

fn cube_vertices() -> Vec<Vertex> {
    CUBE_POSITIONS
        .iter()
        .zip(&CUBE_NORMALS)
        .map(|(&position, &normal)| Vertex { position, normal })
        .collect()
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct InstanceData {
    model: [[f32; 4]; 4],
    normal_matrix: [[f32; 3]; 3],
}

impl InstanceData {
    const ATTRIBUTES: [VertexAttribute; 7] = [
        VertexAttribute {
            offset: 0,
            shader_location: 2,
            format: VertexFormat::Float32x4,
        },
        VertexAttribute {
            offset: size_of::<[f32; 4]>() as BufferAddress,
            shader_location: 3,
            format: VertexFormat::Float32x4,
        },
        VertexAttribute {
            offset: size_of::<[f32; 8]>() as BufferAddress,
            shader_location: 4,
            format: VertexFormat::Float32x4,
        },
        VertexAttribute {
            offset: size_of::<[f32; 12]>() as BufferAddress,
            shader_location: 5,
            format: VertexFormat::Float32x4,
        },
        VertexAttribute {
            offset: size_of::<[f32; 16]>() as BufferAddress,
            shader_location: 6,
            format: VertexFormat::Float32x3,
        },
        VertexAttribute {
            offset: size_of::<[f32; 19]>() as BufferAddress,
            shader_location: 7,
            format: VertexFormat::Float32x3,
        },
        VertexAttribute {
            offset: size_of::<[f32; 22]>() as BufferAddress,
            shader_location: 8,
            format: VertexFormat::Float32x3,
        },
    ];

    fn desc() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: size_of::<InstanceData>() as BufferAddress,
            step_mode: VertexStepMode::Instance,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

#[derive(ShaderType)]
struct ShaderUniforms {
    view_proj: Mat4,
    light_dir: Vec3,
    ambient: f32,
}

const GRID_SIZE: usize = 3;
const NUM_INSTANCES: usize = GRID_SIZE * GRID_SIZE * GRID_SIZE;

fn generate_instances() -> Vec<InstanceData> {
    let mut instances = Vec::with_capacity(NUM_INSTANCES);
    let offset = GRID_SIZE as f32 * 0.5;
    for x in 0..GRID_SIZE {
        for y in 0..GRID_SIZE {
            for z in 0..GRID_SIZE {
                let pos = Vec3::new(
                    x as f32 - offset + 0.5,
                    y as f32 - offset + 0.5,
                    z as f32 - offset + 0.5,
                );
                let model = Mat4::from_translation(pos);
                let nm = Mat3::from_mat4(model.inverse().transpose());
                instances.push(InstanceData {
                    model: model.to_cols_array_2d(),
                    normal_matrix: [
                        nm.x_axis.to_array(),
                        nm.y_axis.to_array(),
                        nm.z_axis.to_array(),
                    ],
                });
            }
        }
    }
    instances
}

struct MSAADemo {
    pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    instance_buffer: Buffer,
    uniform_buffer: Buffer,
    bind_group: BindGroup,
    _msaa_texture: Texture,
    msaa_view: TextureView,
    _depth_texture: Texture,
    depth_texture_view: TextureView,
    camera: Camera,
}

impl MSAADemo {
    fn create_msaa_texture(ctx: &GpuContext) -> (Texture, TextureView) {
        let size = &ctx.surface_config;
        let texture = ctx.device.create_texture(&TextureDescriptor {
            label: Some("MSAA Color Texture"),
            size: Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: SAMPLE_COUNT,
            dimension: TextureDimension::D2,
            format: ctx.surface_format,
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&TextureViewDescriptor::default());
        (texture, view)
    }

    fn create_depth_texture(ctx: &GpuContext) -> (Texture, TextureView) {
        let size = &ctx.surface_config;
        let texture = ctx.device.create_texture(&TextureDescriptor {
            label: Some("MSAA Depth Texture"),
            size: Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: SAMPLE_COUNT,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&TextureViewDescriptor::default());
        (texture, view)
    }
}

impl Example for MSAADemo {
    fn init(ctx: &GpuContext) -> Self {
        let shader = ctx
            .device
            .create_shader_module(include_wgsl!("shader.wgsl"));

        let vertex_buffer = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(&cube_vertices()),
                usage: BufferUsages::VERTEX,
            });
        let index_buffer = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&CUBE_INDICES),
                usage: BufferUsages::INDEX,
            });
        let instances = generate_instances();
        let instance_buffer = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&instances),
                usage: BufferUsages::VERTEX,
            });

        let uniform_buffer = ctx.device.create_buffer(&BufferDescriptor {
            label: Some("Uniform Buffer"),
            size: ShaderUniforms::min_size().into(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bgl = ctx
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Bind Group Layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
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
            layout: &bgl,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = ctx
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Pipeline Layout"),
                bind_group_layouts: &[Some(&bgl)],
                immediate_size: 0,
            });

        let pipeline = ctx
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("MSAA Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[Vertex::desc(), InstanceData::desc()],
                    compilation_options: PipelineCompilationOptions::default(),
                },
                fragment: Some(FragmentState {
                    module: &shader,
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
                    front_face: FrontFace::Ccw,
                    polygon_mode: PolygonMode::Fill,
                    cull_mode: Some(Face::Back),
                    ..Default::default()
                },
                depth_stencil: Some(DepthStencilState {
                    format: TextureFormat::Depth32Float,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(CompareFunction::Less),
                    stencil: StencilState::default(),
                    bias: DepthBiasState::default(),
                }),
                multisample: MultisampleState {
                    count: SAMPLE_COUNT,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                cache: None,
                multiview_mask: None,
            });

        let (msaa_texture, msaa_view) = Self::create_msaa_texture(ctx);
        let (depth_texture, depth_texture_view) = Self::create_depth_texture(ctx);
        let camera = Camera::new(Vec3::new(0.0, 2.0, 6.0), 0.0, -0.25);

        Self {
            pipeline,
            vertex_buffer,
            index_buffer,
            instance_buffer,
            uniform_buffer,
            bind_group,
            _msaa_texture: msaa_texture,
            msaa_view,
            _depth_texture: depth_texture,
            depth_texture_view,
            camera,
        }
    }

    fn resize(&mut self, ctx: &GpuContext, _new_size: PhysicalSize<u32>) {
        let (t, v) = Self::create_msaa_texture(ctx);
        self._msaa_texture = t;
        self.msaa_view = v;
        let (d, dv) = Self::create_depth_texture(ctx);
        self._depth_texture = d;
        self.depth_texture_view = dv;
    }

    fn update(&mut self, _ctx: &GpuContext, dt: Duration, input: &Input) {
        self.camera.update(dt.as_secs_f32(), input);
    }

    fn render(&mut self, ctx: &GpuContext, view: &TextureView, encoder: &mut CommandEncoder) {
        let aspect = ctx.surface_config.width as f32 / ctx.surface_config.height as f32;
        let projection = Mat4::perspective_rh(FRAC_PI_4, aspect, 0.1, 100.0);
        let view_proj = projection * self.camera.view_matrix();

        {
            let mut data = encase::UniformBuffer::new(Vec::new());
            data.write(&ShaderUniforms {
                view_proj,
                light_dir: Vec3::new(-0.5, -1.0, -0.3),
                ambient: 0.1,
            })
            .unwrap();
            ctx.queue
                .write_buffer(&self.uniform_buffer, 0, &data.into_inner());
        }

        let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("MSAA Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &self.msaa_view,
                resolve_target: Some(view),
                ops: Operations {
                    load: LoadOp::Clear(Color::BLACK),
                    store: StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &self.depth_texture_view,
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(1.0),
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        rpass.set_pipeline(&self.pipeline);
        rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        rpass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        rpass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint16);
        rpass.set_bind_group(0, &self.bind_group, &[]);
        rpass.draw_indexed(0..36, 0, 0..NUM_INSTANCES as u32);
    }
}

fn main() {
    run::<MSAADemo>("MSAA");
}
