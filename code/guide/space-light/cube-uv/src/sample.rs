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
    TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureView,
    TextureViewDescriptor, TextureViewDimension, VertexAttribute, VertexBufferLayout, VertexFormat,
    VertexState, VertexStepMode, include_wgsl,
};
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

/// Static camera pose: eye on the face normal, up hint +Y (sides) or +Z (top/bottom).
pub struct FaceView {
    pub eye: Vec3,
    pub up: Vec3,
}

/// Six views in key order 1..6: +Z, -Z, +X, -X, +Y, -Y.
pub const FACE_VIEWS: [FaceView; 6] = [
    FaceView {
        eye: Vec3::new(0.0, 0.0, 3.0),
        up: Vec3::Y,
    },
    FaceView {
        eye: Vec3::new(0.0, 0.0, -3.0),
        up: Vec3::Y,
    },
    FaceView {
        eye: Vec3::new(3.0, 0.0, 0.0),
        up: Vec3::Y,
    },
    FaceView {
        eye: Vec3::new(-3.0, 0.0, 0.0),
        up: Vec3::Y,
    },
    FaceView {
        eye: Vec3::new(0.0, 3.0, 0.0),
        up: Vec3::Z,
    },
    FaceView {
        eye: Vec3::new(0.0, -3.0, 0.0),
        up: Vec3::Z,
    },
];

/// Fixed ortho volume -2..2 x -1.5..1.5; fits the cube, matches the 4:3 frame.
fn ortho() -> Mat4 {
    glam::camera::rh::proj::directx::orthographic(-2.0, 2.0, -1.5, 1.5, 0.1, 10.0)
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    /// Position with w = 1, padded to four components for 4-byte alignment.
    pub position: [f32; 4],
    /// Texture coordinates of this corner inside this face's copy.
    pub uv: [f32; 2],
}

impl Vertex {
    const LAYOUT: VertexBufferLayout<'static> = VertexBufferLayout {
        array_stride: 24,
        step_mode: VertexStepMode::Vertex,
        attributes: &[
            VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: 0,
                shader_location: 0,
            },
            VertexAttribute {
                format: VertexFormat::Float32x2,
                offset: 16,
                shader_location: 1,
            },
        ],
    };
}

/// The 24 cube corners: four records per face, uv (0,0), (1,0), (0,1), (1,1) as seen from outside.
/// Each corner appears in three records with different UV.
pub const VERTICES: [Vertex; 24] = [
    // +Z face: screen right is +X, screen up is +Y.
    Vertex {
        position: [-0.5, 0.5, 0.5, 1.0],
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [0.5, 0.5, 0.5, 1.0],
        uv: [1.0, 0.0],
    },
    Vertex {
        position: [-0.5, -0.5, 0.5, 1.0],
        uv: [0.0, 1.0],
    },
    Vertex {
        position: [0.5, -0.5, 0.5, 1.0],
        uv: [1.0, 1.0],
    },
    // -Z face: screen right is -X, screen up is +Y.
    Vertex {
        position: [0.5, 0.5, -0.5, 1.0],
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [-0.5, 0.5, -0.5, 1.0],
        uv: [1.0, 0.0],
    },
    Vertex {
        position: [0.5, -0.5, -0.5, 1.0],
        uv: [0.0, 1.0],
    },
    Vertex {
        position: [-0.5, -0.5, -0.5, 1.0],
        uv: [1.0, 1.0],
    },
    // +X face: screen right is -Z, screen up is +Y.
    Vertex {
        position: [0.5, 0.5, 0.5, 1.0],
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [0.5, 0.5, -0.5, 1.0],
        uv: [1.0, 0.0],
    },
    Vertex {
        position: [0.5, -0.5, 0.5, 1.0],
        uv: [0.0, 1.0],
    },
    Vertex {
        position: [0.5, -0.5, -0.5, 1.0],
        uv: [1.0, 1.0],
    },
    // -X face: screen right is +Z, screen up is +Y.
    Vertex {
        position: [-0.5, 0.5, -0.5, 1.0],
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [-0.5, 0.5, 0.5, 1.0],
        uv: [1.0, 0.0],
    },
    Vertex {
        position: [-0.5, -0.5, -0.5, 1.0],
        uv: [0.0, 1.0],
    },
    Vertex {
        position: [-0.5, -0.5, 0.5, 1.0],
        uv: [1.0, 1.0],
    },
    // +Y face (top): screen right is -X, screen up is +Z.
    Vertex {
        position: [0.5, 0.5, 0.5, 1.0],
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [-0.5, 0.5, 0.5, 1.0],
        uv: [1.0, 0.0],
    },
    Vertex {
        position: [0.5, 0.5, -0.5, 1.0],
        uv: [0.0, 1.0],
    },
    Vertex {
        position: [-0.5, 0.5, -0.5, 1.0],
        uv: [1.0, 1.0],
    },
    // -Y face (bottom): screen right is +X, screen up is +Z.
    Vertex {
        position: [-0.5, -0.5, 0.5, 1.0],
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [0.5, -0.5, 0.5, 1.0],
        uv: [1.0, 0.0],
    },
    Vertex {
        position: [-0.5, -0.5, -0.5, 1.0],
        uv: [0.0, 1.0],
    },
    Vertex {
        position: [0.5, -0.5, -0.5, 1.0],
        uv: [1.0, 1.0],
    },
];

// Two triangles per face; the shared diagonal keeps the quad flat in UV.
const INDICES: [u16; 36] = [
    0, 1, 2, 2, 1, 3, // +Z
    4, 5, 6, 6, 5, 7, // -Z
    8, 9, 10, 10, 9, 11, // +X
    12, 13, 14, 14, 13, 15, // -X
    16, 17, 18, 18, 17, 19, // +Y
    20, 21, 22, 22, 21, 23, // -Y
];

/// Chapter 22: six explicit quads form a cube; corners repeat per face because the UVs differ.
pub struct CubeUv {
    pipeline: RenderPipeline,
    bind_group: BindGroup,
    view_proj_buffer: Buffer,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    /// Index into FACE_VIEWS; keys 1..6 switch it.
    face: usize,
    depth_texture: Option<Texture>,
    depth_view: Option<TextureView>,
    depth_size: (u32, u32),
    /// Nonzero target size seen in the events; consumed by the next draw.
    pending_size: Option<PhysicalSize<u32>>,
}

impl Sample for CubeUv {
    fn init(gpu: &Gpu) -> Result<Self, Box<dyn Error>> {
        let shader = gpu
            .device
            .create_shader_module(include_wgsl!("shader.wgsl"));
        let layout = gpu
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Cube UV layout"),
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
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: false },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            });
        let pipeline_layout = gpu
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Cube UV pipeline layout"),
                bind_group_layouts: &[Some(&layout)],
                immediate_size: 0,
            });
        let pipeline = gpu
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Cube UV pipeline"),
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
            label: Some("Cube UV view*projection"),
            size: 64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let (_texture, texture_view) = crate::texture::create(gpu);
        let bind_group = gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Cube UV bind group"),
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
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
            ],
        });
        let vertex_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Cube vertices"),
            size: size_of_val(&VERTICES) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&VERTICES));
        let index_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Cube indices"),
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
            vertex_buffer,
            index_buffer,
            face: 0,
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

        let face = &FACE_VIEWS[self.face];
        let view_proj =
            ortho() * glam::camera::rh::view::look_at_mat4(face.eye, Vec3::ZERO, face.up);
        let mut bytes = UniformBuffer::new(Vec::<u8>::new());
        bytes.write(&view_proj).expect("fits the uniform contract");
        gpu.queue
            .write_buffer(&self.view_proj_buffer, 0, &bytes.into_inner());

        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Cube UV pass"),
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
            && let PhysicalKey::Code(key_code) = key_event.physical_key
        {
            let picked = match key_code {
                KeyCode::Digit1 => Some(0),
                KeyCode::Digit2 => Some(1),
                KeyCode::Digit3 => Some(2),
                KeyCode::Digit4 => Some(3),
                KeyCode::Digit5 => Some(4),
                KeyCode::Digit6 => Some(5),
                _ => None,
            };
            if let Some(index) = picked {
                self.face = index;
                // The frame is static: redraw exactly when a key changed it.
                window.request_redraw();
            }
        }
    }
}

impl CubeUv {
    /// Recreates the local depth attachment for a new target size.
    fn recreate_depth(&mut self, gpu: &Gpu, width: u32, height: u32) {
        let texture = gpu.device.create_texture(&TextureDescriptor {
            label: Some("Cube UV depth"),
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
    }

    /// Shows FACE_VIEWS[face]; driven by the verification crate.
    pub fn set_face(&mut self, face: usize) {
        self.face = face;
    }
}
