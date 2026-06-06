#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::f32::consts::FRAC_PI_4;
use std::mem::size_of;
use std::time::Duration;

use bytemuck::{Pod, Zeroable};
use encase::ShaderType;
use glam::{Mat3, Mat4, Vec3};
use wgpu::util::DeviceExt;
use wgpu::{
    include_wgsl, AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry,
    BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType,
    BlendComponent, BlendState, Buffer, BufferAddress, BufferBindingType, BufferDescriptor, BufferUsages,
    Color, ColorTargetState, ColorWrites, CommandEncoder, CompareFunction,
    DepthBiasState, DepthStencilState, Extent3d, Face, FilterMode, FragmentState, FrontFace, IndexFormat,
    LoadOp, MipmapFilterMode, MultisampleState, Operations,
    PipelineCompilationOptions, PipelineLayoutDescriptor, PolygonMode, PrimitiveState,
    PrimitiveTopology, RenderPassColorAttachment, RenderPassDepthStencilAttachment,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, Sampler, SamplerBindingType,
    SamplerDescriptor, ShaderStages, StencilState, StoreOp, Texture, TextureDescriptor,
    TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureView,
    TextureViewDescriptor, TextureViewDimension, VertexAttribute, VertexBufferLayout, VertexFormat,
    VertexState, VertexStepMode,
};
use winit::dpi::PhysicalSize;
use winit::keyboard::KeyCode;

use framework::{
    create_depth_texture, run, Camera, Example, GpuContext, Input, CUBE_INDICES,
    CUBE_NORMALS, CUBE_POSITIONS,
};

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
struct CameraUniforms {
    view_proj: Mat4,
}

#[derive(ShaderType)]
struct LightUniforms {
    light_dir: Vec3,
    ambient: f32,
    light_color: Vec3,
}

#[derive(ShaderType)]
struct PostUniforms {
    mode: u32,
}

const GRID_SIZE: usize = 5;
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

struct RenderToTextureDemo {
    scene_pipeline: RenderPipeline,
    post_pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    instance_buffer: Buffer,
    camera_uniform_buffer: Buffer,
    camera_bind_group: BindGroup,
    light_uniform_buffer: Buffer,
    light_bind_group: BindGroup,
    post_uniform_buffer: Buffer,
    post_bind_group: BindGroup,
    post_bgl: BindGroupLayout,
    sampler: Sampler,
    offscreen_texture: Texture,
    offscreen_view: TextureView,
    offscreen_depth: Texture,
    offscreen_depth_view: TextureView,
    camera: Camera,
    post_mode: u32,
}

impl RenderToTextureDemo {
    const RT_FORMAT: TextureFormat = TextureFormat::Rgba8UnormSrgb;

    fn create_offscreen_texture(ctx: &GpuContext) -> (Texture, TextureView) {
        let size = &ctx.surface_config;
        let texture = ctx.device.create_texture(&TextureDescriptor {
            label: Some("Offscreen Texture"),
            size: Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: Self::RT_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&TextureViewDescriptor::default());
        (texture, view)
    }
}

impl Example for RenderToTextureDemo {
    fn init(ctx: &GpuContext) -> Self {
        let scene_shader = ctx.device.create_shader_module(include_wgsl!("scene.wgsl"));
        let post_shader = ctx.device.create_shader_module(include_wgsl!("post.wgsl"));

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

        // Camera bind group
        let camera_uniform_buffer = ctx.device.create_buffer(&BufferDescriptor {
            label: Some("Camera Uniform"),
            size: CameraUniforms::min_size().into(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_bgl = ctx
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Camera BGL"),
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
            label: Some("Camera BG"),
            layout: &camera_bgl,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: camera_uniform_buffer.as_entire_binding(),
            }],
        });

        // Light bind group
        let light_uniform_buffer = ctx.device.create_buffer(&BufferDescriptor {
            label: Some("Light Uniform"),
            size: LightUniforms::min_size().into(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let light_bgl = ctx
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Light BGL"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(LightUniforms::min_size()),
                    },
                    count: None,
                }],
            });

        let light_bind_group = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Light BG"),
            layout: &light_bgl,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: light_uniform_buffer.as_entire_binding(),
            }],
        });

        // Scene pipeline
        let scene_layout = ctx
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Scene Layout"),
                bind_group_layouts: &[Some(&camera_bgl), Some(&light_bgl)],
                immediate_size: 0,
            });

        let scene_pipeline = ctx
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Scene Pipeline"),
                layout: Some(&scene_layout),
                vertex: VertexState {
                    module: &scene_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[Vertex::desc(), InstanceData::desc()],
                    compilation_options: PipelineCompilationOptions::default(),
                },
                fragment: Some(FragmentState {
                    module: &scene_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(ColorTargetState {
                        format: Self::RT_FORMAT,
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

        // Post-process bind group
        let sampler = ctx.device.create_sampler(&SamplerDescriptor {
            label: Some("Post Sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let post_uniform_buffer = ctx.device.create_buffer(&BufferDescriptor {
            label: Some("Post Uniform"),
            size: PostUniforms::min_size().into(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let (offscreen_texture, offscreen_view) = Self::create_offscreen_texture(ctx);

        let post_bgl = ctx
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Post BGL"),
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
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: Some(PostUniforms::min_size()),
                        },
                        count: None,
                    },
                ],
            });

        let post_bind_group = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Post BG"),
            layout: &post_bgl,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&offscreen_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: post_uniform_buffer.as_entire_binding(),
                },
            ],
        });

        // Post pipeline
        let post_layout = ctx
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Post Layout"),
                bind_group_layouts: &[Some(&post_bgl)],
                immediate_size: 0,
            });

        let post_pipeline = ctx
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Post Pipeline"),
                layout: Some(&post_layout),
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

        let (offscreen_depth, offscreen_depth_view) = create_depth_texture(ctx, "Offscreen Depth");

        let camera = Camera::new(Vec3::new(0.0, 2.0, 8.0), 0.0, -0.2);

        Self {
            scene_pipeline,
            post_pipeline,
            vertex_buffer,
            index_buffer,
            instance_buffer,
            camera_uniform_buffer,
            camera_bind_group,
            light_uniform_buffer,
            light_bind_group,
            post_uniform_buffer,
            post_bind_group,
            post_bgl: post_bgl,
            sampler: sampler,
            offscreen_texture,
            offscreen_view,
            offscreen_depth,
            offscreen_depth_view,
            camera,
            post_mode: 0,
        }
    }

    fn resize(&mut self, ctx: &GpuContext, _new_size: PhysicalSize<u32>) {
        let (offscreen, view) = Self::create_offscreen_texture(ctx);
        self.offscreen_texture = offscreen;
        self.offscreen_view = view;
        let (d, dv) = create_depth_texture(ctx, "Offscreen Depth");
        self.offscreen_depth = d;
        self.offscreen_depth_view = dv;

        self.post_bind_group = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Post BG (resized)"),
            layout: &self.post_bgl,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&self.offscreen_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&self.sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: self.post_uniform_buffer.as_entire_binding(),
                },
            ],
        });
    }

    fn update(&mut self, _ctx: &GpuContext, dt: Duration, input: &Input) {
        self.camera.update(dt.as_secs_f32(), input);
        if input.key_pressed(KeyCode::Digit1) {
            self.post_mode = 0;
        }
        if input.key_pressed(KeyCode::Digit2) {
            self.post_mode = 1;
        }
        if input.key_pressed(KeyCode::Digit3) {
            self.post_mode = 2;
        }
    }

    fn render(&mut self, ctx: &GpuContext, view: &TextureView, encoder: &mut CommandEncoder) {
        let aspect = ctx.surface_config.width as f32 / ctx.surface_config.height as f32;
        let projection = Mat4::perspective_rh(FRAC_PI_4, aspect, 0.1, 100.0);
        let view_mat = self.camera.view_matrix();
        let view_proj = projection * view_mat;

        {
            let mut data = encase::UniformBuffer::new(Vec::new());
            data.write(&CameraUniforms { view_proj }).unwrap();
            ctx.queue
                .write_buffer(&self.camera_uniform_buffer, 0, &data.into_inner());
        }
        {
            let mut data = encase::UniformBuffer::new(Vec::new());
            data.write(&LightUniforms {
                light_dir: Vec3::new(-0.5, -1.0, -0.3),
                ambient: 0.1,
                light_color: Vec3::new(1.0, 0.95, 0.85),
            })
            .unwrap();
            ctx.queue
                .write_buffer(&self.light_uniform_buffer, 0, &data.into_inner());
        }
        {
            let mut data = encase::UniformBuffer::new(Vec::new());
            data.write(&PostUniforms {
                mode: self.post_mode,
            })
            .unwrap();
            ctx.queue
                .write_buffer(&self.post_uniform_buffer, 0, &data.into_inner());
        }

        // Pass 1: scene -> offscreen
        {
            let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Scene Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &self.offscreen_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &self.offscreen_depth_view,
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
            rpass.set_pipeline(&self.scene_pipeline);
            rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            rpass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            rpass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint16);
            rpass.set_bind_group(0, &self.camera_bind_group, &[]);
            rpass.set_bind_group(1, &self.light_bind_group, &[]);
            rpass.draw_indexed(0..36, 0, 0..NUM_INSTANCES as u32);
        }

        // Pass 2: post-process -> screen
        {
            let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Post Pass"),
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
            rpass.set_pipeline(&self.post_pipeline);
            rpass.set_bind_group(0, &self.post_bind_group, &[]);
            rpass.draw(0..6, 0..1);
        }
    }
}

fn main() {
    run::<RenderToTextureDemo>("Render to Texture");
}
