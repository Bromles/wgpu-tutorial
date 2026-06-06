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
    cube_vb: Buffer,
    cube_ib: Buffer,
    floor_vb: Buffer,
    floor_ib: Buffer,
    camera_ub: Buffer,
    camera_bg: BindGroup,
    cube_bg: BindGroup,
    floor_bg: BindGroup,
    scene_tex: Texture,
    scene_tv: TextureView,
    bright_tex: Texture,
    bright_tv: TextureView,
    blur_tex: Texture,
    blur_tv: TextureView,
    bright_params_ub: Buffer,
    bright_bg: BindGroup,
    hblur_bg: BindGroup,
    vblur_bg: BindGroup,
    hblur_params_ub: Buffer,
    vblur_params_ub: Buffer,
    post_bg: BindGroup,
    post_sampler: Sampler,
    depth_tex: Texture,
    depth_tv: TextureView,
    camera: Camera,
}

impl BloomDemo {
    fn create_hdr_tex(
        ctx: &GpuContext,
        label: &str,
        usage: TextureUsages,
    ) -> (Texture, TextureView) {
        let s = &ctx.surface_config;
        let t = ctx.device.create_texture(&TextureDescriptor {
            label: Some(label),
            size: Extent3d {
                width: s.width,
                height: s.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba16Float,
            usage,
            view_formats: &[],
        });
        let v = t.create_view(&TextureViewDescriptor::default());
        (t, v)
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
            label: Some("Compute BGL"),
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

        let cube_vb = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("CubeVB"),
                contents: bytemuck::cast_slice(&cube_vertices()),
                usage: BufferUsages::VERTEX,
            });
        let cube_ib = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("CubeIB"),
                contents: bytemuck::cast_slice(&CUBE_INDICES),
                usage: BufferUsages::INDEX,
            });
        let floor_vb = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("FloorVB"),
                contents: bytemuck::cast_slice(FLOOR_VERTICES),
                usage: BufferUsages::VERTEX,
            });
        let floor_ib = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("FloorIB"),
                contents: bytemuck::cast_slice(FLOOR_INDICES),
                usage: BufferUsages::INDEX,
            });

        let cube_px =
            generate_checkerboard(TEX_SIZE, CELL_SIZE, [180, 60, 60, 255], [100, 35, 35, 255]);
        let cube_tex = ctx.device.create_texture(&TextureDescriptor {
            label: Some("CubeTex"),
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
            cube_tex.as_image_copy(),
            &cube_px,
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
        let cube_tv = cube_tex.create_view(&TextureViewDescriptor::default());

        let floor_px = generate_checkerboard(
            TEX_SIZE,
            CELL_SIZE,
            [200, 200, 200, 255],
            [100, 100, 100, 255],
        );
        let floor_tex = ctx.device.create_texture(&TextureDescriptor {
            label: Some("FloorTex"),
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
            floor_tex.as_image_copy(),
            &floor_px,
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
        let floor_tv = floor_tex.create_view(&TextureViewDescriptor::default());

        let sampler = ctx.device.create_sampler(&SamplerDescriptor {
            label: Some("Sampler"),
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::Repeat,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let camera_ub = ctx.device.create_buffer(&BufferDescriptor {
            label: Some("CameraUB"),
            size: CameraUniforms::min_size().into(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let camera_bgl = ctx
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("CameraBGL"),
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
        let camera_bg = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: Some("CameraBG"),
            layout: &camera_bgl,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: camera_ub.as_entire_binding(),
            }],
        });

        let light_ub = ctx.device.create_buffer(&BufferDescriptor {
            label: Some("LightUB"),
            size: LightUniforms::min_size().into(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        {
            let mut d = encase::UniformBuffer::new(Vec::new());
            d.write(&LightUniforms {
                light_dir: Vec3::new(-1.0, -1.0, -1.0),
                ambient: 0.05,
                intensity: 3.0,
            })
            .unwrap();
            ctx.queue.write_buffer(&light_ub, 0, &d.into_inner());
        }

        let scene_bgl = ctx
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("SceneBGL"),
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
        let cube_bg = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: Some("CubeBG"),
            layout: &scene_bgl,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: light_ub.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&cube_tv),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });
        let floor_bg = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: Some("FloorBG"),
            layout: &scene_bgl,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: light_ub.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&floor_tv),
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
                label: Some("SceneLayout"),
                bind_group_layouts: &[Some(&camera_bgl), Some(&scene_bgl)],
                immediate_size: 0,
            });
        let scene_pipeline = ctx
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("ScenePipe"),
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
        let (scene_tex, scene_tv) = Self::create_hdr_tex(
            ctx,
            "Scene",
            TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        );
        let (bright_tex, bright_tv) = Self::create_hdr_tex(
            ctx,
            "Bright",
            TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
        );
        let (blur_tex, blur_tv) = Self::create_hdr_tex(
            ctx,
            "Blur",
            TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
        );

        // Bright extraction
        let bright_bgl = Self::create_compute_bgl(&ctx.device, true);
        let bright_params_ub = ctx.device.create_buffer(&BufferDescriptor {
            label: Some("BrightParams"),
            size: BrightParams::min_size().into(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        {
            let mut d = encase::UniformBuffer::new(Vec::new());
            d.write(&BrightParams { threshold: 1.0 }).unwrap();
            ctx.queue
                .write_buffer(&bright_params_ub, 0, &d.into_inner());
        }
        let bright_bg = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: Some("BrightBG"),
            layout: &bright_bgl,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&scene_tv),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&bright_tv),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: bright_params_ub.as_entire_binding(),
                },
            ],
        });
        let bright_pipeline = ctx
            .device
            .create_compute_pipeline(&ComputePipelineDescriptor {
                label: Some("BrightPipe"),
                layout: Some(
                    &ctx.device
                        .create_pipeline_layout(&PipelineLayoutDescriptor {
                            label: Some("BrightLayout"),
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
        let hblur_params_ub = ctx.device.create_buffer(&BufferDescriptor {
            label: Some("HBlurParams"),
            size: BlurParams::min_size().into(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let vblur_params_ub = ctx.device.create_buffer(&BufferDescriptor {
            label: Some("VBlurParams"),
            size: BlurParams::min_size().into(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let w = ctx.surface_config.width as f32;
        let h = ctx.surface_config.height as f32;
        {
            let mut d = encase::UniformBuffer::new(Vec::new());
            d.write(&BlurParams {
                direction: glam::Vec2::new(1.0 / w, 0.0),
            })
            .unwrap();
            ctx.queue.write_buffer(&hblur_params_ub, 0, &d.into_inner());
        }
        let hblur_bg = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: Some("HBlurBG"),
            layout: &blur_bgl,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&bright_tv),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&blur_tv),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: hblur_params_ub.as_entire_binding(),
                },
            ],
        });
        {
            let mut d = encase::UniformBuffer::new(Vec::new());
            d.write(&BlurParams {
                direction: glam::Vec2::new(0.0, 1.0 / h),
            })
            .unwrap();
            ctx.queue.write_buffer(&vblur_params_ub, 0, &d.into_inner());
        }
        let vblur_bg = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: Some("VBlurBG"),
            layout: &blur_bgl,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&blur_tv),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&bright_tv),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: vblur_params_ub.as_entire_binding(),
                },
            ],
        });
        let blur_pipeline = ctx
            .device
            .create_compute_pipeline(&ComputePipelineDescriptor {
                label: Some("BlurPipe"),
                layout: Some(
                    &ctx.device
                        .create_pipeline_layout(&PipelineLayoutDescriptor {
                            label: Some("BlurLayout"),
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
            label: Some("PostSampler"),
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            ..Default::default()
        });
        let post_bgl = ctx
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("PostBGL"),
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
        let post_bg = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: Some("PostBG"),
            layout: &post_bgl,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&scene_tv),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&bright_tv),
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
                label: Some("PostPipe"),
                layout: Some(
                    &ctx.device
                        .create_pipeline_layout(&PipelineLayoutDescriptor {
                            label: Some("PostLayout"),
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

        let (depth_tex, depth_tv) = create_depth_texture(ctx, "Depth Texture");
        let camera = Camera::new(Vec3::new(0.0, 2.5, 5.0), 0.0, -0.3);

        Self {
            scene_pipeline,
            bright_pipeline,
            blur_pipeline,
            post_pipeline,
            cube_vb,
            cube_ib,
            floor_vb,
            floor_ib,
            camera_ub,
            camera_bg,
            cube_bg,
            floor_bg,
            scene_tex,
            scene_tv,
            bright_tex,
            bright_tv,
            blur_tex,
            blur_tv,
            bright_params_ub,
            bright_bg,
            hblur_bg,
            vblur_bg,
            hblur_params_ub,
            vblur_params_ub,
            post_bg,
            post_sampler,
            depth_tex,
            depth_tv,
            camera,
        }
    }

    fn resize(&mut self, ctx: &GpuContext, _new_size: PhysicalSize<u32>) {
        let (st, sv) = Self::create_hdr_tex(
            ctx,
            "Scene",
            TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        );
        self.scene_tex = st;
        self.scene_tv = sv;
        let (bt, bv) = Self::create_hdr_tex(
            ctx,
            "Bright",
            TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
        );
        self.bright_tex = bt;
        self.bright_tv = bv;
        let (xt, xv) = Self::create_hdr_tex(
            ctx,
            "Blur",
            TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
        );
        self.blur_tex = xt;
        self.blur_tv = xv;

        let w = ctx.surface_config.width as f32;
        let h = ctx.surface_config.height as f32;

        let bright_bgl = self.bright_pipeline.get_bind_group_layout(0);
        self.bright_bg = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: Some("BrightBG"),
            layout: &bright_bgl,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&self.scene_tv),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&self.bright_tv),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: self.bright_params_ub.as_entire_binding(),
                },
            ],
        });

        let blur_bgl = self.blur_pipeline.get_bind_group_layout(0);
        {
            let mut d = encase::UniformBuffer::new(Vec::new());
            d.write(&BlurParams {
                direction: glam::Vec2::new(1.0 / w, 0.0),
            })
            .unwrap();
            ctx.queue
                .write_buffer(&self.hblur_params_ub, 0, &d.into_inner());
        }
        self.hblur_bg = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: Some("HBlurBG"),
            layout: &blur_bgl,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&self.bright_tv),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&self.blur_tv),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: self.hblur_params_ub.as_entire_binding(),
                },
            ],
        });
        {
            let mut d = encase::UniformBuffer::new(Vec::new());
            d.write(&BlurParams {
                direction: glam::Vec2::new(0.0, 1.0 / h),
            })
            .unwrap();
            ctx.queue
                .write_buffer(&self.vblur_params_ub, 0, &d.into_inner());
        }
        self.vblur_bg = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: Some("VBlurBG"),
            layout: &blur_bgl,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&self.blur_tv),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&self.bright_tv),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: self.vblur_params_ub.as_entire_binding(),
                },
            ],
        });

        let post_bgl = self.post_pipeline.get_bind_group_layout(0);
        self.post_bg = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: Some("PostBG"),
            layout: &post_bgl,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&self.scene_tv),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&self.bright_tv),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(&self.post_sampler),
                },
            ],
        });

        let (d, v) = create_depth_texture(ctx, "Depth Texture");
        self.depth_tex = d;
        self.depth_tv = v;
    }

    fn update(&mut self, _ctx: &GpuContext, dt: Duration, input: &Input) {
        self.camera.update(dt.as_secs_f32(), input);
    }

    fn render(&mut self, ctx: &GpuContext, view: &TextureView, encoder: &mut CommandEncoder) {
        let aspect = ctx.surface_config.width as f32 / ctx.surface_config.height as f32;
        let vp = Mat4::perspective_rh(std::f32::consts::FRAC_PI_4, aspect, 0.1, 100.0)
            * self.camera.view_matrix();
        {
            let mut d = encase::UniformBuffer::new(Vec::new());
            d.write(&CameraUniforms { view_proj: vp }).unwrap();
            ctx.queue.write_buffer(&self.camera_ub, 0, &d.into_inner());
        }

        // 1. Scene → HDR
        {
            let mut r = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Scene"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &self.scene_tv,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &self.depth_tv,
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
            r.set_pipeline(&self.scene_pipeline);
            r.set_bind_group(0, &self.camera_bg, &[]);
            r.set_bind_group(1, &self.cube_bg, &[]);
            r.set_vertex_buffer(0, self.cube_vb.slice(..));
            r.set_index_buffer(self.cube_ib.slice(..), IndexFormat::Uint16);
            r.draw_indexed(0..36, 0, 0..1);
            r.set_bind_group(1, &self.floor_bg, &[]);
            r.set_vertex_buffer(0, self.floor_vb.slice(..));
            r.set_index_buffer(self.floor_ib.slice(..), IndexFormat::Uint16);
            r.draw_indexed(0..6, 0, 0..1);
        }

        let wg_x = (ctx.surface_config.width + 15) / 16;
        let wg_y = (ctx.surface_config.height + 15) / 16;

        // 2. Bright extraction
        {
            let mut c = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("Bright"),
                timestamp_writes: None,
            });
            c.set_pipeline(&self.bright_pipeline);
            c.set_bind_group(0, &self.bright_bg, &[]);
            c.dispatch_workgroups(wg_x, wg_y, 1);
        }

        // 3. H-blur: bright → blur
        {
            let mut c = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("HBlur"),
                timestamp_writes: None,
            });
            c.set_pipeline(&self.blur_pipeline);
            c.set_bind_group(0, &self.hblur_bg, &[]);
            c.dispatch_workgroups(wg_x, wg_y, 1);
        }

        // 4. V-blur: blur → bright
        {
            let mut c = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("VBlur"),
                timestamp_writes: None,
            });
            c.set_pipeline(&self.blur_pipeline);
            c.set_bind_group(0, &self.vblur_bg, &[]);
            c.dispatch_workgroups(wg_x, wg_y, 1);
        }

        // 5. Composite + tone map → screen
        {
            let mut r = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Composite"),
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
            r.set_pipeline(&self.post_pipeline);
            r.set_bind_group(0, &self.post_bg, &[]);
            r.draw(0..6, 0..1);
        }
    }
}

fn main() {
    run::<BloomDemo>("Bloom");
}
