#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::f32::consts::FRAC_PI_4;
use std::mem::size_of;
use std::time::Duration;

use bytemuck::{Pod, Zeroable};
use encase::ShaderType;
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;
use wgpu::{
    include_wgsl, AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, BlendComponent, BlendState,
    Buffer, BufferAddress, BufferBindingType, BufferDescriptor, BufferUsages, Color,
    ColorTargetState, ColorWrites, CommandEncoder, CompareFunction, DepthBiasState, DepthStencilState,
    Extent3d, Face, FilterMode, FragmentState, FrontFace, IndexFormat, LoadOp,
    MipmapFilterMode, MultisampleState, Operations, PipelineCompilationOptions,
    PipelineLayoutDescriptor, PolygonMode, PrimitiveState, PrimitiveTopology,
    RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor,
    RenderPipeline, RenderPipelineDescriptor, SamplerBindingType, SamplerDescriptor, ShaderStages,
    StencilState, StoreOp, TexelCopyBufferLayout, Texture, TextureDescriptor, TextureDimension,
    TextureFormat, TextureSampleType, TextureUsages, TextureView, TextureViewDescriptor,
    TextureViewDimension, VertexAttribute, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
};
use winit::dpi::PhysicalSize;

use framework::{
    create_depth_texture, generate_checkerboard, run, Camera, Example, GpuContext, Input,
    CUBE_INDICES, CUBE_POSITIONS, CUBE_UVS,
};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    position: [f32; 3],
    uv: [f32; 2],
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
            format: VertexFormat::Float32x2,
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
        .zip(&CUBE_UVS)
        .map(|(&position, &uv)| Vertex { position, uv })
        .collect()
}

fn ground_vertices() -> Vec<Vertex> {
    vec![
        Vertex {
            position: [-5.0, 0.0, -5.0],
            uv: [0.0, 0.0],
        },
        Vertex {
            position: [5.0, 0.0, -5.0],
            uv: [10.0, 0.0],
        },
        Vertex {
            position: [5.0, 0.0, 5.0],
            uv: [10.0, 10.0],
        },
        Vertex {
            position: [-5.0, 0.0, 5.0],
            uv: [0.0, 10.0],
        },
    ]
}

const GROUND_INDICES: [u16; 6] = [0, 1, 2, 0, 2, 3];

const TEX_SIZE: u32 = 256;
const CELL_SIZE: u32 = 32;

struct CubeDraw {
    uniform_buffer: Buffer,
    bind_group: BindGroup,
    position: Vec3,
}

#[derive(ShaderType)]
struct ShaderUniforms {
    mvp: Mat4,
}

struct CameraDemo {
    pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    cubes: Vec<CubeDraw>,
    ground_vertex_buffer: Buffer,
    ground_index_buffer: Buffer,
    ground_uniform_buffer: Buffer,
    ground_bind_group: BindGroup,
    depth_texture: Texture,
    depth_texture_view: TextureView,
    camera: Camera,
}

impl Example for CameraDemo {
    fn init(ctx: &GpuContext) -> Self {
        let shader_module = ctx
            .device
            .create_shader_module(include_wgsl!("shader.wgsl"));

        let vertices = cube_vertices();
        let vertex_buffer = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: BufferUsages::VERTEX,
            });

        let index_buffer = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&CUBE_INDICES),
                usage: BufferUsages::INDEX,
            });

        let pixels = generate_checkerboard(
            TEX_SIZE,
            CELL_SIZE,
            [255, 255, 255, 255],
            [58, 134, 173, 255],
        );
        let texture = ctx.device.create_texture(&TextureDescriptor {
            label: Some("Grid Texture"),
            size: Extent3d {
                width: TEX_SIZE,
                height: TEX_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });
        ctx.queue.write_texture(
            texture.as_image_copy(),
            &pixels,
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(TEX_SIZE * 4),
                rows_per_image: Some(TEX_SIZE),
            },
            Extent3d {
                width: TEX_SIZE,
                height: TEX_SIZE,
                depth_or_array_layers: 1,
            },
        );
        let texture_view = texture.create_view(&TextureViewDescriptor::default());

        let sampler = ctx.device.create_sampler(&SamplerDescriptor {
            label: Some("Grid Sampler"),
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::Repeat,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let bind_group_layout = ctx
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Bind Group Layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: Some(ShaderUniforms::min_size()),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let positions = [
            Vec3::new(-1.2, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.2, 0.0, 0.0),
        ];

        let cubes: Vec<CubeDraw> = positions
            .iter()
            .map(|position| {
                let uniform_buffer = ctx.device.create_buffer(&BufferDescriptor {
                    label: Some("Uniform Buffer"),
                    size: ShaderUniforms::min_size().into(),
                    usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });

                let bind_group = ctx.device.create_bind_group(&BindGroupDescriptor {
                    label: Some("Bind Group"),
                    layout: &bind_group_layout,
                    entries: &[
                        BindGroupEntry {
                            binding: 0,
                            resource: uniform_buffer.as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: BindingResource::TextureView(&texture_view),
                        },
                        BindGroupEntry {
                            binding: 2,
                            resource: BindingResource::Sampler(&sampler),
                        },
                    ],
                });

                CubeDraw {
                    uniform_buffer,
                    bind_group,
                    position: *position,
                }
            })
            .collect();

        let ground_verts = ground_vertices();
        let ground_vertex_buffer =
            ctx.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Ground Vertex Buffer"),
                    contents: bytemuck::cast_slice(&ground_verts),
                    usage: BufferUsages::VERTEX,
                });

        let ground_index_buffer =
            ctx.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Ground Index Buffer"),
                    contents: bytemuck::cast_slice(&GROUND_INDICES),
                    usage: BufferUsages::INDEX,
                });

        let ground_uniform_buffer = ctx.device.create_buffer(&BufferDescriptor {
            label: Some("Ground Uniform Buffer"),
            size: ShaderUniforms::min_size().into(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let ground_bind_group = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Ground Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: ground_uniform_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&texture_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
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
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                cache: None,
                multiview_mask: None,
            });

        let (depth_texture, depth_texture_view) = create_depth_texture(ctx, "Depth Texture");

        let camera = Camera::new(Vec3::new(0.0, 1.5, 5.0), 0.0, -0.25);

        Self {
            pipeline,
            vertex_buffer,
            index_buffer,
            cubes,
            ground_vertex_buffer,
            ground_index_buffer,
            ground_uniform_buffer,
            ground_bind_group,
            depth_texture,
            depth_texture_view,
            camera,
        }
    }

    fn resize(&mut self, ctx: &GpuContext, _new_size: PhysicalSize<u32>) {
        let (depth, view) = create_depth_texture(ctx, "Depth Texture");
        self.depth_texture = depth;
        self.depth_texture_view = view;
    }

    fn update(&mut self, _ctx: &GpuContext, dt: Duration, input: &Input) {
        self.camera.update(dt.as_secs_f32(), input);
    }

    fn render(&mut self, ctx: &GpuContext, view: &TextureView, encoder: &mut CommandEncoder) {
        let aspect = ctx.surface_config.width as f32 / ctx.surface_config.height as f32;
        let projection = Mat4::perspective_rh(FRAC_PI_4, aspect, 0.1, 100.0);
        let view_mat = self.camera.view_matrix();

        for cube in &self.cubes {
            let model = Mat4::from_translation(cube.position);
            let mvp = projection * view_mat * model;

            let mut uniform_data = encase::UniformBuffer::new(Vec::new());
            uniform_data.write(&ShaderUniforms { mvp }).unwrap();
            ctx.queue
                .write_buffer(&cube.uniform_buffer, 0, &uniform_data.into_inner());
        }

        {
            let ground_model = Mat4::from_translation(Vec3::new(0.0, -0.5, 0.0));
            let mvp = projection * view_mat * ground_model;
            let mut uniform_data = encase::UniformBuffer::new(Vec::new());
            uniform_data.write(&ShaderUniforms { mvp }).unwrap();
            ctx.queue
                .write_buffer(&self.ground_uniform_buffer, 0, &uniform_data.into_inner());
        }

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
        rpass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint16);

        for cube in &self.cubes {
            rpass.set_bind_group(0, &cube.bind_group, &[]);
            rpass.draw_indexed(0..36, 0, 0..1);
        }

        rpass.set_vertex_buffer(0, self.ground_vertex_buffer.slice(..));
        rpass.set_index_buffer(self.ground_index_buffer.slice(..), IndexFormat::Uint16);
        rpass.set_bind_group(0, &self.ground_bind_group, &[]);
        rpass.draw_indexed(0..6, 0, 0..1);
    }
}

fn main() {
    run::<CameraDemo>("Camera");
}
