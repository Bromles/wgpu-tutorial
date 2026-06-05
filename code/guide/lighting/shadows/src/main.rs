#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};
use std::mem::size_of;
use std::time::Duration;

use bytemuck::{Pod, Zeroable};
use encase::ShaderType;
use glam::{Mat3, Mat4, Vec3};
use wgpu::util::DeviceExt;
use wgpu::{
    include_wgsl, AddressMode, BindGroupDescriptor, BindGroupEntry,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, BlendComponent, BlendState,
    Buffer, BufferAddress, BufferBindingType, BufferDescriptor, BufferUsages, Color,
    ColorTargetState, ColorWrites, CommandEncoder, CompareFunction, DepthBiasState, DepthStencilState,
    Extent3d, FilterMode, FragmentState, IndexFormat, LoadOp, MipmapFilterMode, MultisampleState,
    Operations, PipelineCompilationOptions, PipelineLayoutDescriptor, PolygonMode,
    PrimitiveState, PrimitiveTopology, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor,
    SamplerBindingType, SamplerDescriptor, ShaderStages, StencilState, StoreOp, TexelCopyBufferLayout,
    Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType,
    TextureUsages, TextureView, TextureViewDescriptor, TextureViewDimension, VertexAttribute,
    VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
};
use winit::dpi::PhysicalSize;
use winit::keyboard::KeyCode;

use framework::{run, Example, GpuContext, Input};

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

fn shadow_vertex_layout() -> VertexBufferLayout<'static> {
    const ATTRS: [VertexAttribute; 1] = [VertexAttribute {
        offset: 0,
        shader_location: 0,
        format: VertexFormat::Float32x3,
    }];
    VertexBufferLayout {
        array_stride: size_of::<Vertex>() as BufferAddress,
        step_mode: VertexStepMode::Vertex,
        attributes: &ATTRS,
    }
}

const VERTICES: &[Vertex] = &[
    Vertex {
        position: [-0.5, -0.5, 0.5],
        normal: [0.0, 0.0, 1.0],
        uv: [0.0, 1.0],
    },
    Vertex {
        position: [0.5, -0.5, 0.5],
        normal: [0.0, 0.0, 1.0],
        uv: [1.0, 1.0],
    },
    Vertex {
        position: [0.5, 0.5, 0.5],
        normal: [0.0, 0.0, 1.0],
        uv: [1.0, 0.0],
    },
    Vertex {
        position: [-0.5, 0.5, 0.5],
        normal: [0.0, 0.0, 1.0],
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [0.5, -0.5, -0.5],
        normal: [0.0, 0.0, -1.0],
        uv: [0.0, 1.0],
    },
    Vertex {
        position: [-0.5, -0.5, -0.5],
        normal: [0.0, 0.0, -1.0],
        uv: [1.0, 1.0],
    },
    Vertex {
        position: [-0.5, 0.5, -0.5],
        normal: [0.0, 0.0, -1.0],
        uv: [1.0, 0.0],
    },
    Vertex {
        position: [0.5, 0.5, -0.5],
        normal: [0.0, 0.0, -1.0],
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [0.5, -0.5, 0.5],
        normal: [1.0, 0.0, 0.0],
        uv: [0.0, 1.0],
    },
    Vertex {
        position: [0.5, -0.5, -0.5],
        normal: [1.0, 0.0, 0.0],
        uv: [1.0, 1.0],
    },
    Vertex {
        position: [0.5, 0.5, -0.5],
        normal: [1.0, 0.0, 0.0],
        uv: [1.0, 0.0],
    },
    Vertex {
        position: [0.5, 0.5, 0.5],
        normal: [1.0, 0.0, 0.0],
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [-0.5, -0.5, -0.5],
        normal: [-1.0, 0.0, 0.0],
        uv: [0.0, 1.0],
    },
    Vertex {
        position: [-0.5, -0.5, 0.5],
        normal: [-1.0, 0.0, 0.0],
        uv: [1.0, 1.0],
    },
    Vertex {
        position: [-0.5, 0.5, 0.5],
        normal: [-1.0, 0.0, 0.0],
        uv: [1.0, 0.0],
    },
    Vertex {
        position: [-0.5, 0.5, -0.5],
        normal: [-1.0, 0.0, 0.0],
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [-0.5, 0.5, 0.5],
        normal: [0.0, 1.0, 0.0],
        uv: [0.0, 1.0],
    },
    Vertex {
        position: [0.5, 0.5, 0.5],
        normal: [0.0, 1.0, 0.0],
        uv: [1.0, 1.0],
    },
    Vertex {
        position: [0.5, 0.5, -0.5],
        normal: [0.0, 1.0, 0.0],
        uv: [1.0, 0.0],
    },
    Vertex {
        position: [-0.5, 0.5, -0.5],
        normal: [0.0, 1.0, 0.0],
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [-0.5, -0.5, -0.5],
        normal: [0.0, -1.0, 0.0],
        uv: [0.0, 1.0],
    },
    Vertex {
        position: [0.5, -0.5, -0.5],
        normal: [0.0, -1.0, 0.0],
        uv: [1.0, 1.0],
    },
    Vertex {
        position: [0.5, -0.5, 0.5],
        normal: [0.0, -1.0, 0.0],
        uv: [1.0, 0.0],
    },
    Vertex {
        position: [-0.5, -0.5, 0.5],
        normal: [0.0, -1.0, 0.0],
        uv: [0.0, 0.0],
    },
];

// Floor vertices (flat quad at y = -0.5)
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

const FLOOR_INDICES: &[u16] = &[0, 2, 1, 0, 3, 2];

const INDICES: &[u16] = &[
    0, 1, 2, 2, 3, 0, 4, 5, 6, 6, 7, 4, 8, 9, 10, 10, 11, 8, 12, 13, 14, 14, 15, 12, 16, 17, 18,
    18, 19, 16, 20, 21, 22, 22, 23, 20,
];

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

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ShadowInstanceData {
    model: [[f32; 4]; 4],
}

impl ShadowInstanceData {
    const ATTRIBUTES: [VertexAttribute; 4] = [
        VertexAttribute {
            offset: 0,
            shader_location: 1,
            format: VertexFormat::Float32x4,
        },
        VertexAttribute {
            offset: size_of::<[f32; 4]>() as BufferAddress,
            shader_location: 2,
            format: VertexFormat::Float32x4,
        },
        VertexAttribute {
            offset: size_of::<[f32; 8]>() as BufferAddress,
            shader_location: 3,
            format: VertexFormat::Float32x4,
        },
        VertexAttribute {
            offset: size_of::<[f32; 12]>() as BufferAddress,
            shader_location: 4,
            format: VertexFormat::Float32x4,
        },
    ];

    fn desc() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: size_of::<ShadowInstanceData>() as BufferAddress,
            step_mode: VertexStepMode::Instance,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

struct Camera {
    position: Vec3,
    yaw: f32,
    pitch: f32,
    speed: f32,
    sensitivity: f32,
}

impl Camera {
    fn new(position: Vec3, yaw: f32, pitch: f32) -> Self {
        Self {
            position,
            yaw,
            pitch,
            speed: 5.0,
            sensitivity: 0.003,
        }
    }
    fn direction(&self) -> Vec3 {
        Vec3::new(
            -self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            -self.yaw.cos() * self.pitch.cos(),
        )
    }
    fn forward(&self) -> Vec3 {
        Vec3::new(-self.yaw.sin(), 0.0, -self.yaw.cos())
    }
    fn right(&self) -> Vec3 {
        Vec3::new(self.yaw.cos(), 0.0, -self.yaw.sin())
    }
    fn view_matrix(&self) -> Mat4 {
        Mat4::look_to_rh(self.position, self.direction(), Vec3::Y)
    }

    fn update(&mut self, dt: f32, input: &Input) {
        if input.mouse_button_pressed(1) {
            let (dx, dy) = input.mouse_delta();
            self.yaw -= dx as f32 * self.sensitivity;
            self.pitch -= dy as f32 * self.sensitivity;
            self.pitch = self.pitch.clamp(-FRAC_PI_2 + 0.01, FRAC_PI_2 - 0.01);
        }
        let mut velocity = Vec3::ZERO;
        if input.key_pressed(KeyCode::KeyW) {
            velocity += self.forward();
        }
        if input.key_pressed(KeyCode::KeyS) {
            velocity -= self.forward();
        }
        if input.key_pressed(KeyCode::KeyD) {
            velocity += self.right();
        }
        if input.key_pressed(KeyCode::KeyA) {
            velocity -= self.right();
        }
        if input.key_pressed(KeyCode::Space) {
            velocity.y += 1.0;
        }
        if input.key_pressed(KeyCode::ShiftLeft) {
            velocity.y -= 1.0;
        }
        if velocity.length_squared() > 0.0 {
            self.position += velocity.normalize() * self.speed * dt;
        }
    }
}

#[derive(ShaderType)]
struct CameraUniforms {
    view_proj: Mat4,
}

#[derive(ShaderType)]
struct ShadowLightUniforms {
    light_view_proj: Mat4,
}

#[derive(ShaderType)]
struct SceneLightUniforms {
    light_view_proj: Mat4,
    light_dir: Vec3,
    ambient: f32,
}

const TEX_SIZE: u32 = 256;
const CELL_SIZE: u32 = 32;

fn generate_checkerboard(light: [u8; 4], dark: [u8; 4]) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((TEX_SIZE * TEX_SIZE * 4) as usize);
    for y in 0..TEX_SIZE {
        for x in 0..TEX_SIZE {
            if ((x / CELL_SIZE) + (y / CELL_SIZE)) % 2 == 0 {
                pixels.extend_from_slice(&light);
            } else {
                pixels.extend_from_slice(&dark);
            }
        }
    }
    pixels
}

const CUBE_POSITIONS: &[Vec3] = &[
    Vec3::new(0.0, 0.0, 0.0),
    Vec3::new(-1.5, 0.0, 1.0),
    Vec3::new(1.5, 0.0, -0.5),
];

const SHADOW_MAP_SIZE: u32 = 1024;

struct ShadowsDemo {
    shadow_pipeline: RenderPipeline,
    scene_pipeline: RenderPipeline,
    cube_vertex_buffer: Buffer,
    cube_index_buffer: Buffer,
    floor_vertex_buffer: Buffer,
    floor_index_buffer: Buffer,
    floor_instance_buffer: Buffer,
    instance_buffer: Buffer,
    shadow_instance_buffer: Buffer,
    camera_uniform_buffer: Buffer,
    camera_bind_group: wgpu::BindGroup,
    shadow_light_bind_group: wgpu::BindGroup,
    scene_light_bind_group: wgpu::BindGroup,
    floor_light_bind_group: wgpu::BindGroup,
    _shadow_texture: Texture,
    shadow_texture_view: TextureView,
    depth_texture: Texture,
    depth_texture_view: TextureView,
    camera: Camera,
}

impl ShadowsDemo {
    fn create_shadow_texture(ctx: &GpuContext) -> (Texture, TextureView) {
        let texture = ctx.device.create_texture(&TextureDescriptor {
            label: Some("Shadow Map"),
            size: Extent3d {
                width: SHADOW_MAP_SIZE,
                height: SHADOW_MAP_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&TextureViewDescriptor::default());
        (texture, view)
    }

    fn create_depth_texture(ctx: &GpuContext) -> (Texture, TextureView) {
        let size = &ctx.surface_config;
        let texture = ctx.device.create_texture(&TextureDescriptor {
            label: Some("Depth Texture"),
            size: Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&TextureViewDescriptor::default());
        (texture, view)
    }

    fn light_matrix() -> Mat4 {
        let light_view = Mat4::look_to_rh(
            Vec3::new(3.0, 5.0, 3.0),
            Vec3::new(-1.0, -1.0, -1.0).normalize(),
            Vec3::Y,
        );
        let light_proj = Mat4::orthographic_rh(-6.0, 6.0, -6.0, 6.0, 0.1, 20.0);
        light_proj * light_view
    }
}

impl Example for ShadowsDemo {
    fn init(ctx: &GpuContext) -> Self {
        let shadow_shader = ctx
            .device
            .create_shader_module(include_wgsl!("shadow.wgsl"));
        let scene_shader = ctx.device.create_shader_module(include_wgsl!("scene.wgsl"));

        let cube_vertex_buffer = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Cube Vertex Buffer"),
                contents: bytemuck::cast_slice(VERTICES),
                usage: BufferUsages::VERTEX,
            });
        let cube_index_buffer = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Cube Index Buffer"),
                contents: bytemuck::cast_slice(INDICES),
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
        let floor_instance_buffer =
            ctx.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Floor Instance Buffer"),
                    contents: bytemuck::cast_slice(&[InstanceData {
                        model: Mat4::IDENTITY.to_cols_array_2d(),
                        normal_matrix: Mat3::IDENTITY.to_cols_array_2d(),
                    }]),
                    usage: BufferUsages::VERTEX,
                });

        let instances: Vec<InstanceData> = CUBE_POSITIONS
            .iter()
            .map(|&pos| {
                let model = Mat4::from_translation(pos);
                let normal_matrix = Mat3::from_mat4(model.inverse().transpose());
                InstanceData {
                    model: model.to_cols_array_2d(),
                    normal_matrix: [
                        normal_matrix.x_axis.to_array(),
                        normal_matrix.y_axis.to_array(),
                        normal_matrix.z_axis.to_array(),
                    ],
                }
            })
            .collect();
        let instance_buffer = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&instances),
                usage: BufferUsages::VERTEX,
            });

        let shadow_instances: Vec<ShadowInstanceData> = CUBE_POSITIONS
            .iter()
            .map(|&pos| {
                let model = Mat4::from_translation(pos);
                ShadowInstanceData {
                    model: model.to_cols_array_2d(),
                }
            })
            .collect();
        let shadow_instance_buffer =
            ctx.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Shadow Instance Buffer"),
                    contents: bytemuck::cast_slice(&shadow_instances),
                    usage: BufferUsages::VERTEX,
                });

        // Diffuse texture
        let pixels = generate_checkerboard([180, 60, 60, 255], [100, 35, 35, 255]);
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

        let floor_pixels = generate_checkerboard([200, 200, 200, 255], [100, 100, 100, 255]);
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
        let diffuse_sampler = ctx.device.create_sampler(&SamplerDescriptor {
            label: Some("Diffuse Sampler"),
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::Repeat,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: MipmapFilterMode::Nearest,
            ..Default::default()
        });
        let shadow_sampler = ctx.device.create_sampler(&SamplerDescriptor {
            label: Some("Shadow Sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: MipmapFilterMode::Nearest,
            compare: Some(CompareFunction::LessEqual),
            ..Default::default()
        });

        // Camera (group 0)
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

        // Shadow light (group 0 for shadow pipeline)
        let light_view_proj = Self::light_matrix();
        let shadow_light_uniform_buffer = ctx.device.create_buffer(&BufferDescriptor {
            label: Some("Shadow Light Uniform"),
            size: ShadowLightUniforms::min_size().into(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        {
            let mut data = encase::UniformBuffer::new(Vec::new());
            data.write(&ShadowLightUniforms { light_view_proj })
                .unwrap();
            ctx.queue
                .write_buffer(&shadow_light_uniform_buffer, 0, &data.into_inner());
        }
        let shadow_light_bgl = ctx
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Shadow Light BGL"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(ShadowLightUniforms::min_size()),
                    },
                    count: None,
                }],
            });
        let shadow_light_bind_group = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Shadow Light BG"),
            layout: &shadow_light_bgl,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: shadow_light_uniform_buffer.as_entire_binding(),
            }],
        });

        // Scene light + shadow map + texture (group 1 for scene pipeline)
        let (shadow_texture, shadow_texture_view) = Self::create_shadow_texture(ctx);
        let scene_light_uniform_buffer = ctx.device.create_buffer(&BufferDescriptor {
            label: Some("Scene Light Uniform"),
            size: SceneLightUniforms::min_size().into(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        {
            let mut data = encase::UniformBuffer::new(Vec::new());
            data.write(&SceneLightUniforms {
                light_view_proj,
                light_dir: Vec3::new(-1.0, -1.0, -1.0),
                ambient: 0.15,
            })
            .unwrap();
            ctx.queue
                .write_buffer(&scene_light_uniform_buffer, 0, &data.into_inner());
        }
        let scene_light_bgl = ctx
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Scene Light BGL"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: Some(SceneLightUniforms::min_size()),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Depth,
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Comparison),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        let scene_light_bind_group = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Scene Light BG (cubes)"),
            layout: &scene_light_bgl,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: scene_light_uniform_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&shadow_texture_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(&shadow_sampler),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(&texture_view),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::Sampler(&diffuse_sampler),
                },
            ],
        });
        let floor_light_bind_group = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Scene Light BG (floor)"),
            layout: &scene_light_bgl,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: scene_light_uniform_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&shadow_texture_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(&shadow_sampler),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(&floor_texture_view),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::Sampler(&diffuse_sampler),
                },
            ],
        });

        // Shadow pipeline
        let shadow_layout = ctx
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Shadow Layout"),
                bind_group_layouts: &[Some(&shadow_light_bgl)],
                immediate_size: 0,
            });
        let shadow_pipeline = ctx
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Shadow Pipeline"),
                layout: Some(&shadow_layout),
                vertex: VertexState {
                    module: &shadow_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[shadow_vertex_layout(), ShadowInstanceData::desc()],
                    compilation_options: PipelineCompilationOptions::default(),
                },
                fragment: None,
                primitive: PrimitiveState {
                    topology: PrimitiveTopology::TriangleList,
                    front_face: wgpu::FrontFace::Ccw,
                    polygon_mode: PolygonMode::Fill,
                    cull_mode: Some(wgpu::Face::Back),
                    ..Default::default()
                },
                depth_stencil: Some(DepthStencilState {
                    format: TextureFormat::Depth32Float,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(CompareFunction::Less),
                    stencil: StencilState::default(),
                    bias: DepthBiasState {
                        constant: 2,
                        slope_scale: 2.0,
                        clamp: 0.0,
                    },
                }),
                multisample: MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                cache: None,
                multiview_mask: None,
            });

        // Scene pipeline
        let scene_layout = ctx
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Scene Layout"),
                bind_group_layouts: &[Some(&camera_bgl), Some(&scene_light_bgl)],
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

        let (depth_texture, depth_texture_view) = Self::create_depth_texture(ctx);
        let camera = Camera::new(Vec3::new(0.0, 2.5, 5.0), 0.0, -0.3);

        Self {
            shadow_pipeline,
            scene_pipeline,
            cube_vertex_buffer,
            cube_index_buffer,
            floor_vertex_buffer,
            floor_index_buffer,
            floor_instance_buffer,
            instance_buffer,
            shadow_instance_buffer,
            camera_uniform_buffer,
            camera_bind_group,
            shadow_light_bind_group,
            scene_light_bind_group,
            floor_light_bind_group,
            _shadow_texture: shadow_texture,
            shadow_texture_view,
            depth_texture,
            depth_texture_view,
            camera,
        }
    }

    fn resize(&mut self, ctx: &GpuContext, _new_size: PhysicalSize<u32>) {
        let (d, v) = Self::create_depth_texture(ctx);
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

        {
            let mut data = encase::UniformBuffer::new(Vec::new());
            data.write(&CameraUniforms { view_proj }).unwrap();
            ctx.queue
                .write_buffer(&self.camera_uniform_buffer, 0, &data.into_inner());
        }

        // Pass 1: shadow depth
        {
            let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Shadow Pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &self.shadow_texture_view,
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
            rpass.set_pipeline(&self.shadow_pipeline);
            rpass.set_vertex_buffer(0, self.cube_vertex_buffer.slice(..));
            rpass.set_vertex_buffer(1, self.shadow_instance_buffer.slice(..));
            rpass.set_index_buffer(self.cube_index_buffer.slice(..), IndexFormat::Uint16);
            rpass.set_bind_group(0, &self.shadow_light_bind_group, &[]);
            rpass.draw_indexed(0..36, 0, 0..CUBE_POSITIONS.len() as u32);
        }

        // Pass 2: scene with shadows
        {
            let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Scene Pass"),
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
            rpass.set_pipeline(&self.scene_pipeline);
            rpass.set_bind_group(0, &self.camera_bind_group, &[]);
            rpass.set_bind_group(1, &self.scene_light_bind_group, &[]);

            // Cubes
            rpass.set_vertex_buffer(0, self.cube_vertex_buffer.slice(..));
            rpass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            rpass.set_index_buffer(self.cube_index_buffer.slice(..), IndexFormat::Uint16);
            rpass.draw_indexed(0..36, 0, 0..CUBE_POSITIONS.len() as u32);

            // Floor (single instance, identity matrix)
            rpass.set_vertex_buffer(0, self.floor_vertex_buffer.slice(..));
            rpass.set_vertex_buffer(1, self.floor_instance_buffer.slice(..));
            rpass.set_index_buffer(self.floor_index_buffer.slice(..), IndexFormat::Uint16);
            rpass.set_bind_group(1, &self.floor_light_bind_group, &[]);
            rpass.draw_indexed(0..6, 0, 0..1);
        }
    }
}

fn main() {
    run::<ShadowsDemo>("Shadows");
}
