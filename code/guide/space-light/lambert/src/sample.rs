use std::error::Error;

use encase::UniformBuffer;
use glam::{Mat4, Vec3};
use framework::{Gpu, Sample};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBinding, BufferBindingType, BufferDescriptor,
    BufferSize, BufferUsages, Color, ColorTargetState, ColorWrites, CommandEncoder,
    CompareFunction, DepthStencilState, Extent3d, FragmentState, FrontFace, LoadOp, Operations,
    PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology,
    RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor,
    RenderPipeline, RenderPipelineDescriptor, ShaderStages, StoreOp, Texture, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages, TextureView, TextureViewDescriptor,
    VertexAttribute, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode, include_wgsl,
};
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

use crate::params::LightParams;

/// Fixed camera: five meters in front of the scene, looking at the origin.
const EYE: Vec3 = Vec3::new(0.0, 0.0, 5.0);
const FOV_Y: f32 = std::f32::consts::FRAC_PI_3;
const NEAR: f32 = 0.1;
const FAR: f32 = 50.0;

/// Matches the default window size until the first `Resized` event.
const FALLBACK_SIZE: (u32, u32) = (800, 600);

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    /// Position with w = 1: a point.
    position: [f32; 4],
    /// Normal with w = 0 (a direction), padded to four components for 16-byte alignment.
    normal: [f32; 4],
}

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

/// Slanted quads: a 1.2x1.2 XY square rotated +30/-30 degrees around X.
/// Tilt by theta: (x, y*cos, y*sin) + center, normal (0, -sin, cos).
const QUAD_A_NORMAL: [f32; 4] = [0.0, -0.5, 0.8660254, 0.0];
const QUAD_B_NORMAL: [f32; 4] = [0.0, 0.5, 0.8660254, 0.0];
/// Split face: coplanar halves sharing the seam x = 0; the crease is the normal attribute alone.
const SPLIT_LEFT_NORMAL: [f32; 4] = [
    0.0,
    -std::f32::consts::FRAC_1_SQRT_2,
    std::f32::consts::FRAC_1_SQRT_2,
    0.0,
];
const SPLIT_RIGHT_NORMAL: [f32; 4] = [0.0, 0.0, 1.0, 0.0];

const VERTICES: [Vertex; 16] = [
    // Quad A: tilt +30 degrees, center (-0.95, 0.55, 0).
    Vertex {
        position: [-1.55, 1.0696, 0.3, 1.0],
        normal: QUAD_A_NORMAL,
    },
    Vertex {
        position: [-0.35, 1.0696, 0.3, 1.0],
        normal: QUAD_A_NORMAL,
    },
    Vertex {
        position: [-1.55, 0.0304, -0.3, 1.0],
        normal: QUAD_A_NORMAL,
    },
    Vertex {
        position: [-0.35, 0.0304, -0.3, 1.0],
        normal: QUAD_A_NORMAL,
    },
    // Quad B: tilt -30 degrees, center (0.95, 0.55, 0).
    Vertex {
        position: [0.35, 1.0696, -0.3, 1.0],
        normal: QUAD_B_NORMAL,
    },
    Vertex {
        position: [1.55, 1.0696, -0.3, 1.0],
        normal: QUAD_B_NORMAL,
    },
    Vertex {
        position: [0.35, 0.0304, 0.3, 1.0],
        normal: QUAD_B_NORMAL,
    },
    Vertex {
        position: [1.55, 0.0304, 0.3, 1.0],
        normal: QUAD_B_NORMAL,
    },
    // Split face, left half: x in [-1, 0], normal tilted -45 degrees.
    Vertex {
        position: [-1.0, -0.3, 0.0, 1.0],
        normal: SPLIT_LEFT_NORMAL,
    },
    Vertex {
        position: [0.0, -0.3, 0.0, 1.0],
        normal: SPLIT_LEFT_NORMAL,
    },
    Vertex {
        position: [-1.0, -1.3, 0.0, 1.0],
        normal: SPLIT_LEFT_NORMAL,
    },
    Vertex {
        position: [0.0, -1.3, 0.0, 1.0],
        normal: SPLIT_LEFT_NORMAL,
    },
    // Split face, right half: x in [0, 1], normal facing the camera.
    Vertex {
        position: [0.0, -0.3, 0.0, 1.0],
        normal: SPLIT_RIGHT_NORMAL,
    },
    Vertex {
        position: [1.0, -0.3, 0.0, 1.0],
        normal: SPLIT_RIGHT_NORMAL,
    },
    Vertex {
        position: [0.0, -1.3, 0.0, 1.0],
        normal: SPLIT_RIGHT_NORMAL,
    },
    Vertex {
        position: [1.0, -1.3, 0.0, 1.0],
        normal: SPLIT_RIGHT_NORMAL,
    },
];

// Two triangles per quad; the shared diagonal keeps each quad flat.
const INDICES: [u16; 24] = [
    0, 1, 2, 2, 1, 3, // quad A
    4, 5, 6, 6, 5, 7, // quad B
    8, 9, 10, 10, 9, 11, // split, left half
    12, 13, 14, 14, 13, 15, // split, right half
];

/// Chapter 23: flat faces with explicit normals; N switches normal-as-color and Lambert.
pub struct Lambert {
    pipeline: RenderPipeline,
    bind_group: BindGroup,
    view_proj_buffer: Buffer,
    params_buffer: Buffer,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    params: LightParams,
    projection: Mat4,
    depth_texture: Option<Texture>,
    depth_view: Option<TextureView>,
    depth_size: (u32, u32),
    /// Nonzero target size seen in the events; consumed by the next draw.
    pending_size: Option<PhysicalSize<u32>>,
}

impl Sample for Lambert {
    fn init(gpu: &Gpu) -> Result<Self, Box<dyn Error>> {
        let shader = gpu
            .device
            .create_shader_module(include_wgsl!("shader.wgsl"));
        let layout = gpu
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Lambert layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(64),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(48),
                        },
                        count: None,
                    },
                ],
            });
        let pipeline_layout = gpu
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Lambert pipeline layout"),
                bind_group_layouts: &[Some(&layout)],
                immediate_size: 0,
            });
        let pipeline = gpu
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Lambert pipeline"),
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
                depth_stencil: Some(DepthStencilState {
                    format: TextureFormat::Depth32Float,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(CompareFunction::Less),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                cache: None,
                multiview_mask: None,
            });
        let view_proj_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Lambert view*projection"),
            size: 64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let params_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Lambert light params"),
            size: 48,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Lambert bind group"),
            layout: &layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(BufferBinding {
                        buffer: &view_proj_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(BufferBinding {
                        buffer: &params_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });
        let vertex_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Face vertices"),
            size: size_of_val(&VERTICES) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&VERTICES));
        let index_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Face indices"),
            size: size_of_val(&INDICES) as u64,
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&index_buffer, 0, bytemuck::cast_slice(&INDICES));
        Ok(Self {
            pipeline,
            bind_group,
            view_proj_buffer,
            params_buffer,
            vertex_buffer,
            index_buffer,
            params: LightParams::chapter(),
            projection: glam::camera::rh::proj::directx::perspective(
                FOV_Y,
                FALLBACK_SIZE.0 as f32 / FALLBACK_SIZE.1 as f32,
                NEAR,
                FAR,
            ),
            depth_texture: None,
            depth_view: None,
            depth_size: (0, 0),
            pending_size: None,
        })
    }

    /// Called once after init and on every resize; the offscreen harness calls it before its draw.
    fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.pending_size = Some(PhysicalSize::new(width, height));
        }
    }

    fn draw(&mut self, gpu: &Gpu, encoder: &mut CommandEncoder, view: &TextureView) {
        if let Some(size) = self.pending_size.take()
            && self.depth_size != (size.width, size.height)
        {
            self.recreate_depth(gpu, size.width, size.height);
        }

        // Model is the identity: view*projection is the whole chain.
        let view_proj =
            self.projection * glam::camera::rh::view::look_at_mat4(EYE, Vec3::ZERO, Vec3::Y);
        let mut matrix_bytes = UniformBuffer::new(Vec::<u8>::new());
        matrix_bytes
            .write(&view_proj)
            .expect("fits the uniform contract");
        gpu.queue
            .write_buffer(&self.view_proj_buffer, 0, &matrix_bytes.into_inner());
        let mut params_bytes = UniformBuffer::new(Vec::<u8>::new());
        params_bytes
            .write(&self.params)
            .expect("fits the uniform contract");
        gpu.queue
            .write_buffer(&self.params_buffer, 0, &params_bytes.into_inner());

        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Lambert pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color {
                        r: 0.1,
                        g: 0.1,
                        b: 0.12,
                        a: 1.0,
                    }),
                    store: StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: self
                    .depth_view
                    .as_ref()
                    .expect("depth exists after recreate"),
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(1.0),
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..RenderPassDescriptor::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        pass.draw_indexed(0..INDICES.len() as u32, 0, 0..1);
    }

    fn window_event(&mut self, window: &winit::window::Window, event: &WindowEvent) {
        if let WindowEvent::KeyboardInput {
            event: key_event, ..
        } = event
            && key_event.state == ElementState::Pressed
            && let PhysicalKey::Code(KeyCode::KeyN) = key_event.physical_key
        {
            // 1 -> 0 -> 1: flip the mode flag and repaint once.
            self.params.show_normals = 1 - self.params.show_normals;
            window.request_redraw();
        }
    }
}

impl Lambert {
    /// Recreates the depth attachment and projection for a new target size.
    fn recreate_depth(&mut self, gpu: &Gpu, width: u32, height: u32) {
        let texture = gpu.device.create_texture(&TextureDescriptor {
            label: Some("Lambert depth"),
            size: Extent3d {
                width,
                height,
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
        self.depth_texture = Some(texture);
        self.depth_view = Some(view);
        self.depth_size = (width, height);
        let aspect = width as f32 / height as f32;
        self.projection =
            glam::camera::rh::proj::directx::perspective(FOV_Y, aspect, NEAR, FAR);
    }

    /// Selects the display mode; the verification crate drives this directly.
    pub fn set_show_normals(&mut self, show: bool) {
        self.params.show_normals = if show { 1 } else { 0 };
    }
}
