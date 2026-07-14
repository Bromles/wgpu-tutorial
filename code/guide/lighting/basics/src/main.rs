#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::f32::consts::FRAC_PI_4;
use std::mem::size_of;
use std::time::Duration;

use bytemuck::{Pod, Zeroable};
use encase::ShaderType;
use glam::{Mat3, Mat4, Vec3};
use wgpu::util::DeviceExt;
use wgpu::{
    AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, BlendComponent, BlendState, Buffer,
    BufferAddress, BufferBindingType, BufferDescriptor, BufferUsages, Color, ColorTargetState,
    ColorWrites, CommandEncoder, CompareFunction, DepthBiasState, DepthStencilState, Extent3d,
    Face, FilterMode, FragmentState, FrontFace, IndexFormat, LoadOp, MipmapFilterMode,
    MultisampleState, Operations, PipelineCompilationOptions, PipelineLayoutDescriptor,
    PolygonMode, PrimitiveState, PrimitiveTopology, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, SamplerBindingType, SamplerDescriptor, ShaderStages, StencilState,
    StoreOp, TexelCopyBufferLayout, Texture, TextureDescriptor, TextureDimension, TextureFormat,
    TextureSampleType, TextureUsages, TextureView, TextureViewDescriptor, TextureViewDimension,
    VertexAttribute, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode, include_wgsl,
};
use winit::dpi::PhysicalSize;

use framework::{
    CUBE_INDICES, CUBE_NORMALS, CUBE_POSITIONS, CUBE_UVS, Camera, Example, GpuContext, Input,
    create_depth_texture, generate_checkerboard, run,
};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
}

impl Vertex {
    const ATTRIBUTES: [VertexAttribute; 3] = [
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
        VertexAttribute {
            offset: size_of::<[f32; 6]>() as BufferAddress,
            shader_location: 2,
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
        .zip(&CUBE_NORMALS)
        .zip(&CUBE_UVS)
        .map(|((&position, &normal), &uv)| Vertex {
            position,
            normal,
            uv,
        })
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
            shader_location: 3,
            format: VertexFormat::Float32x4,
        },
        VertexAttribute {
            offset: size_of::<[f32; 4]>() as BufferAddress,
            shader_location: 4,
            format: VertexFormat::Float32x4,
        },
        VertexAttribute {
            offset: size_of::<[f32; 8]>() as BufferAddress,
            shader_location: 5,
            format: VertexFormat::Float32x4,
        },
        VertexAttribute {
            offset: size_of::<[f32; 12]>() as BufferAddress,
            shader_location: 6,
            format: VertexFormat::Float32x4,
        },
        VertexAttribute {
            offset: size_of::<[f32; 16]>() as BufferAddress,
            shader_location: 7,
            format: VertexFormat::Float32x3,
        },
        VertexAttribute {
            offset: size_of::<[f32; 19]>() as BufferAddress,
            shader_location: 8,
            format: VertexFormat::Float32x3,
        },
        VertexAttribute {
            offset: size_of::<[f32; 22]>() as BufferAddress,
            shader_location: 9,
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
}

#[derive(ShaderType)]
struct LightUniforms {
    light_dir: Vec3,
    ambient: f32,
    light_color: Vec3,
}

const GRID_SIZE: usize = 5;
const NUM_INSTANCES: usize = GRID_SIZE * GRID_SIZE * GRID_SIZE;
const TEX_SIZE: u32 = 256;
const CELL_SIZE: u32 = 32;

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
                let normal_matrix = Mat3::from_mat4(model.inverse().transpose());
                instances.push(InstanceData {
                    model: model.to_cols_array_2d(),
                    normal_matrix: [
                        normal_matrix.x_axis.to_array(),
                        normal_matrix.y_axis.to_array(),
                        normal_matrix.z_axis.to_array(),
                    ],
                });
            }
        }
    }
    instances
}

struct LightingDemo {
    pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    instance_buffer: Buffer,
    camera_uniform_buffer: Buffer,
    camera_bind_group: BindGroup,
    #[expect(dead_code)]
    light_uniform_buffer: Buffer,
    light_bind_group: BindGroup,
    depth_texture: Texture,
    depth_texture_view: TextureView,
    camera: Camera,
}

impl Example for LightingDemo {
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

        let instances = generate_instances();
        let instance_buffer = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&instances),
                usage: BufferUsages::VERTEX,
            });

        let pixels = generate_checkerboard(
            TEX_SIZE,
            CELL_SIZE,
            [180, 180, 180, 255],
            [100, 100, 100, 255],
        );
        let texture = ctx.device.create_texture(&TextureDescriptor {
            label: Some("Diffuse Texture"),
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
            label: Some("Diffuse Sampler"),
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::Repeat,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let camera_uniform_buffer = ctx.device.create_buffer(&BufferDescriptor {
            label: Some("Camera Uniform Buffer"),
            size: ShaderUniforms::min_size().into(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_bind_group_layout =
            ctx.device
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: Some("Camera Bind Group Layout"),
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

        let camera_bind_group = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: camera_uniform_buffer.as_entire_binding(),
            }],
        });

        let light_uniform_buffer = ctx.device.create_buffer(&BufferDescriptor {
            label: Some("Light Uniform Buffer"),
            size: LightUniforms::min_size().into(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        {
            let mut light_data = encase::UniformBuffer::new(Vec::new());
            light_data
                .write(&LightUniforms {
                    light_dir: Vec3::new(-0.5, -1.0, -0.3),
                    ambient: 0.1,
                    light_color: Vec3::new(1.0, 0.95, 0.85),
                })
                .unwrap();
            ctx.queue
                .write_buffer(&light_uniform_buffer, 0, &light_data.into_inner());
        }

        let light_bind_group_layout =
            ctx.device
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: Some("Light Bind Group Layout"),
                    entries: &[
                        BindGroupLayoutEntry {
                            binding: 0,
                            visibility: ShaderStages::FRAGMENT,
                            ty: BindingType::Buffer {
                                ty: BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: Some(LightUniforms::min_size()),
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

        let light_bind_group = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Light Bind Group"),
            layout: &light_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: light_uniform_buffer.as_entire_binding(),
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
                bind_group_layouts: &[
                    Some(&camera_bind_group_layout),
                    Some(&light_bind_group_layout),
                ],
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
                    buffers: &[Some(Vertex::desc()), Some(InstanceData::desc())],
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

        let camera = Camera::new(Vec3::new(0.0, 2.0, 8.0), 0.0, -0.2);

        Self {
            pipeline,
            vertex_buffer,
            index_buffer,
            instance_buffer,
            camera_uniform_buffer,
            camera_bind_group,
            light_uniform_buffer,
            light_bind_group,
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
        let projection =
            glam::camera::rh::proj::directx::perspective(FRAC_PI_4, aspect, 0.1, 100.0);
        let view_mat = self.camera.view_matrix();
        let view_proj = projection * view_mat;

        {
            let mut uniform_data = encase::UniformBuffer::new(Vec::new());
            uniform_data
                .write(&ShaderUniforms { view_proj })
                .expect("Failed to write uniform buffer");
            ctx.queue
                .write_buffer(&self.camera_uniform_buffer, 0, &uniform_data.into_inner());
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
        rpass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        rpass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint16);
        rpass.set_bind_group(0, &self.camera_bind_group, &[]);
        rpass.set_bind_group(1, &self.light_bind_group, &[]);
        rpass.draw_indexed(0..36, 0, 0..NUM_INSTANCES as u32);
    }
}

fn main() {
    run::<LightingDemo>("Lighting Basics");
}
