#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::f32::consts::{FRAC_PI_4, PI};
use std::mem::size_of;
use std::time::Duration;

use bytemuck::{Pod, Zeroable};
use encase::ShaderType;
use glam::{Mat3, Mat4, Vec3, Vec4};
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

fn generate_sphere(stacks: u32, slices: u32, radius: f32) -> (Vec<Vertex>, Vec<u16>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for stack in 0..=stacks {
        let phi = PI * stack as f32 / stacks as f32;
        let sin_phi = phi.sin();
        let cos_phi = phi.cos();

        for slice in 0..=slices {
            let theta = 2.0 * PI * slice as f32 / slices as f32;
            let sin_theta = theta.sin();
            let cos_theta = theta.cos();

            let x = cos_theta * sin_phi;
            let y = cos_phi;
            let z = sin_theta * sin_phi;

            vertices.push(Vertex {
                position: [x * radius, y * radius, z * radius],
                normal: [x, y, z],
                uv: [slice as f32 / slices as f32, stack as f32 / stacks as f32],
            });
        }
    }

    for stack in 0..stacks {
        for slice in 0..slices {
            let a = (stack * (slices + 1) + slice) as u16;
            let b = a + slices as u16 + 1;
            indices.push(a);
            indices.push(b);
            indices.push(a + 1);
            indices.push(a + 1);
            indices.push(b);
            indices.push(b + 1);
        }
    }

    (vertices, indices)
}

const TEX_SIZE: u32 = 256;
const CELL_SIZE: u32 = 32;

#[derive(ShaderType)]
struct CameraUniforms {
    view_proj: Mat4,
}

#[derive(ShaderType)]
struct MeshUniforms {
    model: Mat4,
    normal_matrix: Mat3,
    light_dir: Vec3,
    ambient: f32,
    base_color: Vec4,
}

struct MeshDraw {
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    index_count: u32,
    bind_group: BindGroup,
    model: Mat4,
    normal_matrix: Mat3,
    uniform_buffer: Buffer,
    base_color: Vec4,
}

struct ModelLoadingDemo {
    pipeline: RenderPipeline,
    meshes: Vec<MeshDraw>,
    depth_texture: Texture,
    depth_texture_view: TextureView,
    camera: Camera,
    camera_uniform_buffer: Buffer,
    camera_bind_group: BindGroup,
}

impl Example for ModelLoadingDemo {
    fn init(ctx: &GpuContext) -> Self {
        let shader = ctx
            .device
            .create_shader_module(include_wgsl!("shader.wgsl"));

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

        let mesh_bgl = ctx
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Mesh BGL"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: Some(MeshUniforms::min_size()),
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

        let pipeline_layout = ctx
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Pipeline Layout"),
                bind_group_layouts: &[Some(&camera_bgl), Some(&mesh_bgl)],
                immediate_size: 0,
            });

        let pipeline = ctx
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[Vertex::desc()],
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
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                cache: None,
                multiview_mask: None,
            });

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

        let camera_uniform_buffer = ctx.device.create_buffer(&BufferDescriptor {
            label: Some("Camera Uniform"),
            size: CameraUniforms::min_size().into(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_bind_group = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Camera BG"),
            layout: &camera_bgl,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: camera_uniform_buffer.as_entire_binding(),
            }],
        });

        let create_texture = |r: u8, g: u8, b: u8| {
            let pixels =
                generate_checkerboard(TEX_SIZE, CELL_SIZE, [255, 255, 255, 255], [r, g, b, 255]);
            let tex = ctx.device.create_texture(&TextureDescriptor {
                label: Some("Mesh Texture"),
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
                tex.as_image_copy(),
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
            tex.create_view(&TextureViewDescriptor::default())
        };

        let create_mesh = |vertices: &[Vertex],
                           indices: &[u16],
                           tex_view: &TextureView,
                           color: Vec4,
                           model: Mat4| {
            let vertex_buffer = ctx
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Mesh VB"),
                    contents: bytemuck::cast_slice(vertices),
                    usage: BufferUsages::VERTEX,
                });
            let index_buffer = ctx
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Mesh IB"),
                    contents: bytemuck::cast_slice(indices),
                    usage: BufferUsages::INDEX,
                });
            let uniform_buffer = ctx.device.create_buffer(&BufferDescriptor {
                label: Some("Mesh Uniform"),
                size: MeshUniforms::min_size().into(),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let nm = Mat3::from_mat4(model.inverse().transpose());
            let bind_group = ctx.device.create_bind_group(&BindGroupDescriptor {
                label: Some("Mesh BG"),
                layout: &mesh_bgl,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: uniform_buffer.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::TextureView(tex_view),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::Sampler(&sampler),
                    },
                ],
            });
            MeshDraw {
                vertex_buffer,
                index_buffer,
                index_count: indices.len() as u32,
                bind_group,
                model,
                normal_matrix: nm,
                uniform_buffer,
                base_color: color,
            }
        };

        let sphere = generate_sphere(16, 32, 1.0);
        let tex1 = create_texture(58, 134, 173);
        let tex2 = create_texture(173, 58, 58);
        let tex3 = create_texture(58, 173, 87);

        let meshes = vec![
            create_mesh(
                &sphere.0,
                &sphere.1,
                &tex1,
                Vec4::new(1.0, 1.0, 1.0, 1.0),
                Mat4::from_translation(Vec3::new(-3.0, 0.0, 0.0)),
            ),
            create_mesh(
                &sphere.0,
                &sphere.1,
                &tex2,
                Vec4::new(1.0, 1.0, 1.0, 1.0),
                Mat4::from_translation(Vec3::new(0.0, 0.0, 0.0)),
            ),
            create_mesh(
                &sphere.0,
                &sphere.1,
                &tex3,
                Vec4::new(1.0, 1.0, 1.0, 1.0),
                Mat4::from_translation(Vec3::new(3.0, 0.0, 0.0)),
            ),
        ];

        let (depth_texture, depth_texture_view) = create_depth_texture(ctx, "Depth Texture");
        let camera = Camera::new(Vec3::new(0.0, 2.0, 7.0), 0.0, -0.2);

        Self {
            pipeline,
            meshes,
            depth_texture,
            depth_texture_view,
            camera,
            camera_uniform_buffer,
            camera_bind_group,
        }
    }

    fn resize(&mut self, ctx: &GpuContext, _new_size: PhysicalSize<u32>) {
        let (d, v) = create_depth_texture(ctx, "Depth Texture");
        self.depth_texture = d;
        self.depth_texture_view = v;
    }

    fn update(&mut self, _ctx: &GpuContext, dt: Duration, input: &Input) {
        self.camera.update(dt.as_secs_f32(), input);
    }

    fn render(&mut self, ctx: &GpuContext, view: &TextureView, encoder: &mut CommandEncoder) {
        let aspect = ctx.surface_config.width as f32 / ctx.surface_config.height as f32;
        let projection = Mat4::perspective_rh(FRAC_PI_4, aspect, 0.1, 100.0);
        let view_proj = projection * self.camera.view_matrix();

        let mut camera_data = encase::UniformBuffer::new(Vec::new());
        camera_data.write(&CameraUniforms { view_proj }).unwrap();
        ctx.queue
            .write_buffer(&self.camera_uniform_buffer, 0, &camera_data.into_inner());

        for mesh in &self.meshes {
            let mut data = encase::UniformBuffer::new(Vec::new());
            data.write(&MeshUniforms {
                model: mesh.model,
                normal_matrix: mesh.normal_matrix,
                light_dir: Vec3::new(-0.5, -1.0, -0.3),
                ambient: 0.1,
                base_color: mesh.base_color,
            })
            .unwrap();
            ctx.queue
                .write_buffer(&mesh.uniform_buffer, 0, &data.into_inner());
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
        rpass.set_bind_group(0, &self.camera_bind_group, &[]);
        for mesh in &self.meshes {
            rpass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            rpass.set_index_buffer(mesh.index_buffer.slice(..), IndexFormat::Uint16);
            rpass.set_bind_group(1, &mesh.bind_group, &[]);
            rpass.draw_indexed(0..mesh.index_count, 0, 0..1);
        }
    }
}

fn main() {
    run::<ModelLoadingDemo>("Multiple Meshes");
}
