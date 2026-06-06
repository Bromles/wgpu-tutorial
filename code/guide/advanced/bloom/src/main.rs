#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::mem::size_of;
use std::time::Duration;

use bytemuck::{Pod, Zeroable};
use encase::ShaderType;
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;
use wgpu::{
    include_wgsl, AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry,
    BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType,
    BlendComponent, BlendState, Buffer, BufferAddress, BufferBindingType, BufferDescriptor, BufferUsages,
    Color, ColorTargetState, ColorWrites, CommandEncoder, CompareFunction,
    ComputePassDescriptor, ComputePipeline, ComputePipelineDescriptor, DepthStencilState, Device, Extent3d,
    Face, FilterMode, FragmentState, FrontFace, IndexFormat, LoadOp, MipmapFilterMode,
    MultisampleState, Operations, PipelineCompilationOptions, PipelineLayoutDescriptor, PolygonMode,
    PrimitiveState, PrimitiveTopology, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, Sampler,
    SamplerBindingType, SamplerDescriptor, ShaderStages, StencilState, StorageTextureAccess,
    StoreOp, TexelCopyBufferLayout, Texture, TextureDescriptor, TextureDimension,
    TextureFormat, TextureSampleType, TextureUsages, TextureView, TextureViewDescriptor,
    TextureViewDimension, VertexAttribute, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
};
use winit::dpi::PhysicalSize;

use framework::{
    create_depth_texture, generate_checkerboard, run, Camera, Example, GpuContext, Input, CUBE_INDICES,
    CUBE_NORMALS, CUBE_POSITIONS, CUBE_UVS,
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

const FLOOR_VERTICES: &[Vertex] = &[
    Vertex {
        position: [-5.0, -0.5, -5.0],
        normal: [0.0, 1.0, 0.0],
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [5.0, -0.5, -5.0],
        normal: [0.0, 1.0, 0.0],
        uv: [5.0, 0.0],
    },
    Vertex {
        position: [5.0, -0.5, 5.0],
        normal: [0.0, 1.0, 0.0],
        uv: [5.0, 5.0],
    },
    Vertex {
        position: [-5.0, -0.5, 5.0],
        normal: [0.0, 1.0, 0.0],
        uv: [0.0, 5.0],
    },
];
const FLOOR_INDICES: &[u16] = &[0, 1, 2, 0, 2, 3];

#[derive(ShaderType)]
struct CameraUniforms {
    view_proj: Mat4,
}

#[derive(ShaderType)]
struct LightUniforms {
    light_dir: Vec3,
    ambient: f32,
    intensity: f32,
}

#[derive(ShaderType)]
struct BrightParams {
    threshold: f32,
}

#[derive(ShaderType)]
struct BlurParams {
    direction: glam::Vec2,
}

const TEX_SIZE: u32 = 256;
const CELL_SIZE: u32 = 32;

struct BloomDemo {
    scene_pipeline: RenderPipeline,
    bright_pipeline: ComputePipeline,
    blur_pipeline: ComputePipeline,
    post_pipeline: RenderPipeline,
    cube_vertex_buffer: Buffer,
    cube_index_buffer: Buffer,
    floor_vertex_buffer: Buffer,
    floor_index_buffer: Buffer,
    camera_uniform_buffer: Buffer,
    camera_bind_group: BindGroup,
    cube_bind_group: BindGroup,
    floor_bind_group: BindGroup,
    scene_texture: Texture,
    scene_texture_view: TextureView,
    bright_texture: Texture,
    bright_texture_view: TextureView,
    blur_texture: Texture,
    blur_texture_view: TextureView,
    bright_params_buffer: Buffer,
    bright_bind_group: BindGroup,
    hblur_bind_group: BindGroup,
    vblur_bind_group: BindGroup,
    hblur_params_buffer: Buffer,
    vblur_params_buffer: Buffer,
    post_bind_group: BindGroup,
    post_sampler: Sampler,
    depth_texture: Texture,
    depth_texture_view: TextureView,
    camera: Camera,
}

impl BloomDemo {
    fn create_hdr_tex(
        ctx: &GpuContext,
        label: &str,
        usage: TextureUsages,
    ) -> (Texture, TextureView) {
        let config = &ctx.surface_config;
        let texture = ctx.device.create_texture(&TextureDescriptor {
            label: Some(label),
            size: Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba16Float,
            usage,
            view_formats: &[],
        });
        let view = texture.create_view(&TextureViewDescriptor::default());
        (texture, view)
    }
    fn create_compute_bgl(device: &Device, extra_buffer: bool) -> BindGroupLayout {
        let mut entries = vec![
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: false },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::StorageTexture {
                    access: StorageTextureAccess::WriteOnly,
                    format: TextureFormat::Rgba16Float,
                    view_dimension: TextureViewDimension::D2,
                },
                count: None,
            },
        ];
        if extra_buffer {
            entries.push(BindGroupLayoutEntry {
                binding: 2,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            });
        }
        device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Compute Bind Group Layout"),
            entries: &entries,
        })
    }
}

impl Example for BloomDemo {
    fn init(ctx: &GpuContext) -> Self {
        let scene_shader = ctx.device.create_shader_module(include_wgsl!("scene.wgsl"));
        let bright_shader = ctx
            .device
            .create_shader_module(include_wgsl!("bright.wgsl"));
        let blur_shader = ctx.device.create_shader_module(include_wgsl!("blur.wgsl"));
        let post_shader = ctx.device.create_shader_module(include_wgsl!("post.wgsl"));

        let cube_vertex_buffer = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Cube Vertex Buffer"),
                contents: bytemuck::cast_slice(&cube_vertices()),
                usage: BufferUsages::VERTEX,
            });
        let cube_index_buffer = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Cube Index Buffer"),
                contents: bytemuck::cast_slice(&CUBE_INDICES),
                usage: BufferUsages::INDEX,
            });
        let floor_vertex_buffer =
            ctx.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Floor Vertex Buffer"),
                    contents: bytemuck::cast_slice(FLOOR_VERTICES),
                    usage: BufferUsages::VERTEX,
                });
        let floor_index_buffer = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Floor Index Buffer"),
                contents: bytemuck::cast_slice(FLOOR_INDICES),
                usage: BufferUsages::INDEX,
            });

        let cube_pixels =
            generate_checkerboard(TEX_SIZE, CELL_SIZE, [180, 60, 60, 255], [100, 35, 35, 255]);
        let cube_texture = ctx.device.create_texture(&TextureDescriptor {
            label: Some("Cube Texture"),
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
            cube_texture.as_image_copy(),
            &cube_pixels,
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
        let cube_texture_view = cube_texture.create_view(&TextureViewDescriptor::default());

        let floor_pixels = generate_checkerboard(
            TEX_SIZE,
            CELL_SIZE,
            [200, 200, 200, 255],
            [100, 100, 100, 255],
        );
        let floor_texture = ctx.device.create_texture(&TextureDescriptor {
            label: Some("Floor Texture"),
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
            floor_texture.as_image_copy(),
            &floor_pixels,
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
        let floor_texture_view = floor_texture.create_view(&TextureViewDescriptor::default());

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
            size: CameraUniforms::min_size().into(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let camera_bgl = ctx
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Camera Bind Group Layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(CameraUniforms::min_size()),
                    },
                    count: None,
                }],
            });
        let camera_bind_group = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bgl,
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
            let mut data = encase::UniformBuffer::new(Vec::new());
            data.write(&LightUniforms {
                light_dir: Vec3::new(-1.0, -1.0, -1.0),
                ambient: 0.05,
                intensity: 3.0,
            })
            .unwrap();
            ctx.queue
                .write_buffer(&light_uniform_buffer, 0, &data.into_inner());
        }

        let scene_bgl = ctx
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Scene Bind Group Layout"),
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
        let cube_bind_group = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Cube Bind Group"),
            layout: &scene_bgl,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: light_uniform_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&cube_texture_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });
        let floor_bind_group = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Floor Bind Group"),
            layout: &scene_bgl,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: light_uniform_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&floor_texture_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });

        let scene_layout = ctx
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Scene Pipeline Layout"),
                bind_group_layouts: &[Some(&camera_bgl), Some(&scene_bgl)],
                immediate_size: 0,
            });
        let scene_pipeline = ctx
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Scene Render Pipeline"),
                layout: Some(&scene_layout),
                vertex: VertexState {
                    module: &scene_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[Vertex::desc()],
                    compilation_options: PipelineCompilationOptions::default(),
                },
                fragment: Some(FragmentState {
                    module: &scene_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(ColorTargetState {
                        format: TextureFormat::Rgba16Float,
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
                    bias: Default::default(),
                }),
                multisample: MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                cache: None,
                multiview_mask: None,
            });

        // Textures
        let (scene_texture, scene_texture_view) = Self::create_hdr_tex(
            ctx,
            "Scene Texture",
            TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        );
        let (bright_texture, bright_texture_view) = Self::create_hdr_tex(
            ctx,
            "Bright Texture",
            TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
        );
        let (blur_texture, blur_texture_view) = Self::create_hdr_tex(
            ctx,
            "Blur Texture",
            TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
        );

        // Bright extraction
        let bright_bgl = Self::create_compute_bgl(&ctx.device, true);
        let bright_params_buffer = ctx.device.create_buffer(&BufferDescriptor {
            label: Some("Bright Parameters Buffer"),
            size: BrightParams::min_size().into(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        {
            let mut data = encase::UniformBuffer::new(Vec::new());
            data.write(&BrightParams { threshold: 1.0 })
                .expect("Failed to write uniform buffer");
            ctx.queue
                .write_buffer(&bright_params_buffer, 0, &data.into_inner());
        }
        let bright_bind_group = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Bright Bind Group"),
            layout: &bright_bgl,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&scene_texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&bright_texture_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: bright_params_buffer.as_entire_binding(),
                },
            ],
        });
        let bright_pipeline = ctx
            .device
            .create_compute_pipeline(&ComputePipelineDescriptor {
                label: Some("Bright Compute Pipeline"),
                layout: Some(
                    &ctx.device
                        .create_pipeline_layout(&PipelineLayoutDescriptor {
                            label: Some("Bright Pipeline Layout"),
                            bind_group_layouts: &[Some(&bright_bgl)],
                            immediate_size: 0,
                        }),
                ),
                module: &bright_shader,
                entry_point: Some("main"),
                compilation_options: PipelineCompilationOptions::default(),
                cache: None,
            });

        // Blur
        let blur_bgl = Self::create_compute_bgl(&ctx.device, true);
        let hblur_params_buffer = ctx.device.create_buffer(&BufferDescriptor {
            label: Some("Horizontal Blur Parameters Buffer"),
            size: BlurParams::min_size().into(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let vblur_params_buffer = ctx.device.create_buffer(&BufferDescriptor {
            label: Some("Vertical Blur Parameters Buffer"),
            size: BlurParams::min_size().into(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let width = ctx.surface_config.width as f32;
        let height = ctx.surface_config.height as f32;
        {
            let mut data = encase::UniformBuffer::new(Vec::new());
            data.write(&BlurParams {
                direction: glam::Vec2::new(1.0 / width, 0.0),
            })
            .unwrap();
            ctx.queue
                .write_buffer(&hblur_params_buffer, 0, &data.into_inner());
        }
        let hblur_bind_group = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Horizontal Blur Bind Group"),
            layout: &blur_bgl,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&bright_texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&blur_texture_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: hblur_params_buffer.as_entire_binding(),
                },
            ],
        });
        {
            let mut data = encase::UniformBuffer::new(Vec::new());
            data.write(&BlurParams {
                direction: glam::Vec2::new(0.0, 1.0 / height),
            })
            .unwrap();
            ctx.queue
                .write_buffer(&vblur_params_buffer, 0, &data.into_inner());
        }
        let vblur_bind_group = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Vertical Blur Bind Group"),
            layout: &blur_bgl,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&blur_texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&bright_texture_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: vblur_params_buffer.as_entire_binding(),
                },
            ],
        });
        let blur_pipeline = ctx
            .device
            .create_compute_pipeline(&ComputePipelineDescriptor {
                label: Some("Blur Compute Pipeline"),
                layout: Some(
                    &ctx.device
                        .create_pipeline_layout(&PipelineLayoutDescriptor {
                            label: Some("Blur Pipeline Layout"),
                            bind_group_layouts: &[Some(&blur_bgl)],
                            immediate_size: 0,
                        }),
                ),
                module: &blur_shader,
                entry_point: Some("main"),
                compilation_options: PipelineCompilationOptions::default(),
                cache: None,
            });

        // Post (scene + bloom → screen)
        let post_sampler = ctx.device.create_sampler(&SamplerDescriptor {
            label: Some("Post Process Sampler"),
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            ..Default::default()
        });
        let post_bgl = ctx
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Post Process Bind Group Layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
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
        let post_bind_group = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Post Process Bind Group"),
            layout: &post_bgl,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&scene_texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&bright_texture_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(&post_sampler),
                },
            ],
        });
        let post_pipeline = ctx
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Post Process Render Pipeline"),
                layout: Some(
                    &ctx.device
                        .create_pipeline_layout(&PipelineLayoutDescriptor {
                            label: Some("Post Process Pipeline Layout"),
                            bind_group_layouts: &[Some(&post_bgl)],
                            immediate_size: 0,
                        }),
                ),
                vertex: VertexState {
                    module: &post_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: PipelineCompilationOptions::default(),
                },
                fragment: Some(FragmentState {
                    module: &post_shader,
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

        let (depth_texture, depth_texture_view) = create_depth_texture(ctx, "Depth Texture");
        let camera = Camera::new(Vec3::new(0.0, 2.5, 5.0), 0.0, -0.3);

        Self {
            scene_pipeline,
            bright_pipeline,
            blur_pipeline,
            post_pipeline,
            cube_vertex_buffer,
            cube_index_buffer,
            floor_vertex_buffer,
            floor_index_buffer,
            camera_uniform_buffer,
            camera_bind_group,
            cube_bind_group,
            floor_bind_group,
            scene_texture,
            scene_texture_view,
            bright_texture,
            bright_texture_view,
            blur_texture,
            blur_texture_view,
            bright_params_buffer,
            bright_bind_group,
            hblur_bind_group,
            vblur_bind_group,
            hblur_params_buffer,
            vblur_params_buffer,
            post_bind_group,
            post_sampler,
            depth_texture,
            depth_texture_view,
            camera,
        }
    }

    fn resize(&mut self, ctx: &GpuContext, _new_size: PhysicalSize<u32>) {
        let (scene_texture, scene_texture_view) = Self::create_hdr_tex(
            ctx,
            "Scene Texture",
            TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        );
        self.scene_texture = scene_texture;
        self.scene_texture_view = scene_texture_view;
        let (bright_texture, bright_texture_view) = Self::create_hdr_tex(
            ctx,
            "Bright Texture",
            TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
        );
        self.bright_texture = bright_texture;
        self.bright_texture_view = bright_texture_view;
        let (blur_texture, blur_texture_view) = Self::create_hdr_tex(
            ctx,
            "Blur Texture",
            TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
        );
        self.blur_texture = blur_texture;
        self.blur_texture_view = blur_texture_view;

        let width = ctx.surface_config.width as f32;
        let height = ctx.surface_config.height as f32;

        let bright_bgl = self.bright_pipeline.get_bind_group_layout(0);
        self.bright_bind_group = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Bright Bind Group"),
            layout: &bright_bgl,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&self.scene_texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&self.bright_texture_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: self.bright_params_buffer.as_entire_binding(),
                },
            ],
        });

        let blur_bgl = self.blur_pipeline.get_bind_group_layout(0);
        {
            let mut data = encase::UniformBuffer::new(Vec::new());
            data.write(&BlurParams {
                direction: glam::Vec2::new(1.0 / width, 0.0),
            })
            .unwrap();
            ctx.queue
                .write_buffer(&self.hblur_params_buffer, 0, &data.into_inner());
        }
        self.hblur_bind_group = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Horizontal Blur Bind Group"),
            layout: &blur_bgl,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&self.bright_texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&self.blur_texture_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: self.hblur_params_buffer.as_entire_binding(),
                },
            ],
        });
        {
            let mut data = encase::UniformBuffer::new(Vec::new());
            data.write(&BlurParams {
                direction: glam::Vec2::new(0.0, 1.0 / height),
            })
            .unwrap();
            ctx.queue
                .write_buffer(&self.vblur_params_buffer, 0, &data.into_inner());
        }
        self.vblur_bind_group = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Vertical Blur Bind Group"),
            layout: &blur_bgl,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&self.blur_texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&self.bright_texture_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: self.vblur_params_buffer.as_entire_binding(),
                },
            ],
        });

        let post_bgl = self.post_pipeline.get_bind_group_layout(0);
        self.post_bind_group = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Post Process Bind Group"),
            layout: &post_bgl,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&self.scene_texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&self.bright_texture_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(&self.post_sampler),
                },
            ],
        });

        let (depth_texture, depth_texture_view) = create_depth_texture(ctx, "Depth Texture");
        self.depth_texture = depth_texture;
        self.depth_texture_view = depth_texture_view;
    }

    fn update(&mut self, _ctx: &GpuContext, dt: Duration, input: &Input) {
        self.camera.update(dt.as_secs_f32(), input);
    }

    fn render(&mut self, ctx: &GpuContext, view: &TextureView, encoder: &mut CommandEncoder) {
        let aspect = ctx.surface_config.width as f32 / ctx.surface_config.height as f32;
        let view_proj = Mat4::perspective_rh(std::f32::consts::FRAC_PI_4, aspect, 0.1, 100.0)
            * self.camera.view_matrix();
        {
            let mut data = encase::UniformBuffer::new(Vec::new());
            data.write(&CameraUniforms {
                view_proj: view_proj,
            })
            .expect("Failed to write uniform buffer");
            ctx.queue
                .write_buffer(&self.camera_uniform_buffer, 0, &data.into_inner());
        }

        // 1. Scene → HDR
        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Scene Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &self.scene_texture_view,
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
            render_pass.set_pipeline(&self.scene_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.cube_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.cube_vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.cube_index_buffer.slice(..), IndexFormat::Uint16);
            render_pass.draw_indexed(0..36, 0, 0..1);
            render_pass.set_bind_group(1, &self.floor_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.floor_vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.floor_index_buffer.slice(..), IndexFormat::Uint16);
            render_pass.draw_indexed(0..6, 0, 0..1);
        }

        let workgroup_x = (ctx.surface_config.width + 15) / 16;
        let workgroup_y = (ctx.surface_config.height + 15) / 16;

        // 2. Bright extraction
        {
            let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("Bright Extraction Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.bright_pipeline);
            compute_pass.set_bind_group(0, &self.bright_bind_group, &[]);
            compute_pass.dispatch_workgroups(workgroup_x, workgroup_y, 1);
        }

        // 3. H-blur: bright → blur
        {
            let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("Horizontal Blur Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.blur_pipeline);
            compute_pass.set_bind_group(0, &self.hblur_bind_group, &[]);
            compute_pass.dispatch_workgroups(workgroup_x, workgroup_y, 1);
        }

        // 4. V-blur: blur → bright
        {
            let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("Vertical Blur Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.blur_pipeline);
            compute_pass.set_bind_group(0, &self.vblur_bind_group, &[]);
            compute_pass.dispatch_workgroups(workgroup_x, workgroup_y, 1);
        }

        // 5. Composite + tone map → screen
        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Composite Render Pass"),
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
            render_pass.set_pipeline(&self.post_pipeline);
            render_pass.set_bind_group(0, &self.post_bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }
    }
}

fn main() {
    run::<BloomDemo>("Bloom");
}
