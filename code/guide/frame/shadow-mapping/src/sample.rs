use std::error::Error;

use encase::UniformBuffer;
use glam::Mat4;
use shell::{Gpu, Sample};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
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

use crate::params::SceneParams;
use crate::scene;

/// 1024x1024 over the 6x6 m box gives ~5.9 mm per texel.
const SHADOW_SIZE: u32 = 1024;
/// The M key cycle for the cube X offset: rest, +1, -1.
const CUBE_OFFSETS: [f32; 3] = [0.0, 1.0, -1.0];

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    /// Position with w = 1: a point.
    position: [f32; 4],
    /// Normal with w = 0; padded to four components for alignment.
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

/// The +Y normal of the floor quad, repeated per corner.
const FLOOR_NORMAL: [f32; 4] = [0.0, 1.0, 0.0, 0.0];

/// Floor receiver and cube caster in one indexed mesh; the cube center
/// sits at (0, 0.51, 0) so the shadow keeps a contact region.
const VERTICES: [Vertex; 28] = [
    // Floor: 4x4 m quad at y = 0, as seen from above.
    Vertex {
        position: [-2.0, 0.0, -2.0, 1.0],
        normal: FLOOR_NORMAL,
    },
    Vertex {
        position: [2.0, 0.0, -2.0, 1.0],
        normal: FLOOR_NORMAL,
    },
    Vertex {
        position: [-2.0, 0.0, 2.0, 1.0],
        normal: FLOOR_NORMAL,
    },
    Vertex {
        position: [2.0, 0.0, 2.0, 1.0],
        normal: FLOOR_NORMAL,
    },
    // Cube +Z face.
    Vertex {
        position: [-0.5, 1.01, 0.5, 1.0],
        normal: [0.0, 0.0, 1.0, 0.0],
    },
    Vertex {
        position: [0.5, 1.01, 0.5, 1.0],
        normal: [0.0, 0.0, 1.0, 0.0],
    },
    Vertex {
        position: [-0.5, 0.01, 0.5, 1.0],
        normal: [0.0, 0.0, 1.0, 0.0],
    },
    Vertex {
        position: [0.5, 0.01, 0.5, 1.0],
        normal: [0.0, 0.0, 1.0, 0.0],
    },
    // Cube -Z face.
    Vertex {
        position: [0.5, 1.01, -0.5, 1.0],
        normal: [0.0, 0.0, -1.0, 0.0],
    },
    Vertex {
        position: [-0.5, 1.01, -0.5, 1.0],
        normal: [0.0, 0.0, -1.0, 0.0],
    },
    Vertex {
        position: [0.5, 0.01, -0.5, 1.0],
        normal: [0.0, 0.0, -1.0, 0.0],
    },
    Vertex {
        position: [-0.5, 0.01, -0.5, 1.0],
        normal: [0.0, 0.0, -1.0, 0.0],
    },
    // Cube +X face.
    Vertex {
        position: [0.5, 1.01, 0.5, 1.0],
        normal: [1.0, 0.0, 0.0, 0.0],
    },
    Vertex {
        position: [0.5, 1.01, -0.5, 1.0],
        normal: [1.0, 0.0, 0.0, 0.0],
    },
    Vertex {
        position: [0.5, 0.01, 0.5, 1.0],
        normal: [1.0, 0.0, 0.0, 0.0],
    },
    Vertex {
        position: [0.5, 0.01, -0.5, 1.0],
        normal: [1.0, 0.0, 0.0, 0.0],
    },
    // Cube -X face.
    Vertex {
        position: [-0.5, 1.01, -0.5, 1.0],
        normal: [-1.0, 0.0, 0.0, 0.0],
    },
    Vertex {
        position: [-0.5, 1.01, 0.5, 1.0],
        normal: [-1.0, 0.0, 0.0, 0.0],
    },
    Vertex {
        position: [-0.5, 0.01, -0.5, 1.0],
        normal: [-1.0, 0.0, 0.0, 0.0],
    },
    Vertex {
        position: [-0.5, 0.01, 0.5, 1.0],
        normal: [-1.0, 0.0, 0.0, 0.0],
    },
    // Cube +Y face (top).
    Vertex {
        position: [0.5, 1.01, 0.5, 1.0],
        normal: [0.0, 1.0, 0.0, 0.0],
    },
    Vertex {
        position: [-0.5, 1.01, 0.5, 1.0],
        normal: [0.0, 1.0, 0.0, 0.0],
    },
    Vertex {
        position: [0.5, 1.01, -0.5, 1.0],
        normal: [0.0, 1.0, 0.0, 0.0],
    },
    Vertex {
        position: [-0.5, 1.01, -0.5, 1.0],
        normal: [0.0, 1.0, 0.0, 0.0],
    },
    // Cube -Y face (bottom).
    Vertex {
        position: [-0.5, 0.01, 0.5, 1.0],
        normal: [0.0, -1.0, 0.0, 0.0],
    },
    Vertex {
        position: [0.5, 0.01, 0.5, 1.0],
        normal: [0.0, -1.0, 0.0, 0.0],
    },
    Vertex {
        position: [-0.5, 0.01, -0.5, 1.0],
        normal: [0.0, -1.0, 0.0, 0.0],
    },
    Vertex {
        position: [0.5, 0.01, -0.5, 1.0],
        normal: [0.0, -1.0, 0.0, 0.0],
    },
];

/// Two triangles per quad, the shared-diagonal pattern of the guide.
const INDICES: [u16; 42] = [
    0, 1, 2, 2, 1, 3, 4, 5, 6, 6, 5, 7, 8, 9, 10, 10, 9, 11, 12, 13, 14, 14, 13, 15, 16, 17, 18,
    18, 17, 19, 20, 21, 22, 22, 21, 23, 24, 25, 26, 26, 25, 27,
];

/// Draw ranges inside INDICES: one mesh, the passes pick their parts.
const FLOOR_INDICES: std::ops::Range<u32> = 0..6;
const CUBE_INDICES: std::ops::Range<u32> = 6..42;

/// A depth pass renders the cube into a light-space depth map; the main
/// pass scales the direct term by the per-fragment lookup.
pub struct ShadowMapping {
    depth_pipeline: RenderPipeline,
    main_pipeline: RenderPipeline,
    /// Model = identity: the floor never moves.
    floor_bind_group: BindGroup,
    /// Model = the M-dependent cube translation.
    cube_main_bind_group: BindGroup,
    /// Depth-pass group: model + light matrix; no shadow map bound.
    cube_shadow_bind_group: BindGroup,
    camera_buffer: Buffer,
    light_buffer: Buffer,
    params_buffer: Buffer,
    floor_model_buffer: Buffer,
    cube_model_buffer: Buffer,
    params: SceneParams,
    /// Index into CUBE_OFFSETS; the M key cycles it.
    cube_offset: usize,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    shadow_view: TextureView,
    depth_texture: Option<Texture>,
    depth_view: Option<TextureView>,
    depth_size: (u32, u32),
    /// Nonzero target size seen in the events; consumed by the next draw.
    pending_size: Option<PhysicalSize<u32>>,
}

impl Sample for ShadowMapping {
    fn init(gpu: &Gpu) -> Result<Self, Box<dyn Error>> {
        let shader = gpu
            .device
            .create_shader_module(include_wgsl!("shader.wgsl"));
        let main_layout = gpu
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Shadow mapping main layout"),
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
                        visibility: ShaderStages::VERTEX,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(64),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(64),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(48),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility: ShaderStages::FRAGMENT,
                        // A depth texture: textureLoad returns the single f32.
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Depth,
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            });
        // Uses only slots 0 and 2; numbers match the shader bindings.
        let depth_layout = gpu
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Shadow mapping depth layout"),
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
                        binding: 2,
                        visibility: ShaderStages::VERTEX,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(64),
                        },
                        count: None,
                    },
                ],
            });
        let main_pipeline_layout = gpu
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Shadow mapping main pipeline layout"),
                bind_group_layouts: &[Some(&main_layout)],
                immediate_size: 0,
            });
        let depth_pipeline_layout = gpu
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Shadow mapping depth pipeline layout"),
                bind_group_layouts: &[Some(&depth_layout)],
                immediate_size: 0,
            });
        let depth_pipeline = gpu
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Shadow depth pipeline"),
                layout: Some(&depth_pipeline_layout),
                vertex: VertexState {
                    module: &shader,
                    entry_point: Some("vs_depth"),
                    buffers: &[Some(Vertex::LAYOUT)],
                    compilation_options: PipelineCompilationOptions::default(),
                },
                // No fragment stage: depth only.
                fragment: None,
                primitive: PrimitiveState {
                    topology: PrimitiveTopology::TriangleList,
                    front_face: FrontFace::Ccw,
                    cull_mode: None,
                    ..PrimitiveState::default()
                },
                // The pipeline must declare the attachment's depth format.
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
        let main_pipeline = gpu
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Shadow mapping main pipeline"),
                layout: Some(&main_pipeline_layout),
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
                // Depth test: the closer surface wins regardless of draw order.
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

        let camera_buffer = create_uniform(gpu, "Camera view*projection", 64);
        let light_buffer = create_uniform(gpu, "Light view*projection", 64);
        let params_buffer = create_uniform(gpu, "Shadow mapping scene params", 48);
        let shared_buffers = MainBuffers {
            layout: &main_layout,
            camera: &camera_buffer,
            light: &light_buffer,
            params: &params_buffer,
        };
        let floor_model_buffer = create_uniform(gpu, "Floor model (identity)", 64);
        let cube_model_buffer = create_uniform(gpu, "Cube model (translation)", 64);

        // Attachment in pass 1, texture in pass 2; fixed size.
        let shadow_texture = gpu.device.create_texture(&TextureDescriptor {
            label: Some("Shadow map"),
            size: Extent3d {
                width: SHADOW_SIZE,
                height: SHADOW_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let shadow_view = shadow_texture.create_view(&TextureViewDescriptor {
            label: Some("Shadow map view"),
            ..TextureViewDescriptor::default()
        });

        let cube_shadow_bind_group = gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Cube caster bind group"),
            layout: &depth_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(BufferBinding {
                        buffer: &cube_model_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(BufferBinding {
                        buffer: &light_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });
        let floor_bind_group = create_main_bind_group(
            gpu,
            "Floor main bind group",
            &floor_model_buffer,
            &shared_buffers,
            &shadow_view,
        );
        let cube_main_bind_group = create_main_bind_group(
            gpu,
            "Cube main bind group",
            &cube_model_buffer,
            &shared_buffers,
            &shadow_view,
        );

        let vertex_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Floor and cube vertices"),
            size: size_of_val(&VERTICES) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&VERTICES));
        let index_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Floor and cube indices"),
            size: size_of_val(&INDICES) as u64,
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&index_buffer, 0, bytemuck::cast_slice(&INDICES));

        Ok(Self {
            depth_pipeline,
            main_pipeline,
            floor_bind_group,
            cube_main_bind_group,
            cube_shadow_bind_group,
            camera_buffer,
            light_buffer,
            params_buffer,
            floor_model_buffer,
            cube_model_buffer,
            params: SceneParams::chapter(),
            cube_offset: 0,
            vertex_buffer,
            index_buffer,
            shadow_view,
            depth_texture: None,
            depth_view: None,
            depth_size: (0, 0),
            pending_size: None,
        })
    }

    /// Shell contract: called right after `init` and on every resize.
    fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.pending_size = Some(PhysicalSize::new(width, height));
        }
    }

    fn draw(&mut self, gpu: &Gpu, encoder: &mut CommandEncoder, view: &TextureView) {
        // A pending size (re)creates the depth attachment here, in draw.
        if let Some(size) = self.pending_size.take()
            && self.depth_size != (size.width, size.height)
        {
            self.recreate_depth(gpu, size.width, size.height);
        }

        // Fixed cameras; only the cube model follows the M key.
        let mut bytes = UniformBuffer::new(Vec::<u8>::new());
        bytes
            .write(&scene::camera_view_proj())
            .expect("fits the uniform contract");
        gpu.queue
            .write_buffer(&self.camera_buffer, 0, &bytes.into_inner());
        let mut bytes = UniformBuffer::new(Vec::<u8>::new());
        bytes
            .write(&scene::light_view_proj())
            .expect("fits the uniform contract");
        gpu.queue
            .write_buffer(&self.light_buffer, 0, &bytes.into_inner());
        let mut bytes = UniformBuffer::new(Vec::<u8>::new());
        bytes
            .write(&self.params)
            .expect("fits the uniform contract");
        gpu.queue
            .write_buffer(&self.params_buffer, 0, &bytes.into_inner());
        let mut bytes = UniformBuffer::new(Vec::<u8>::new());
        bytes
            .write(&Mat4::IDENTITY)
            .expect("fits the uniform contract");
        gpu.queue
            .write_buffer(&self.floor_model_buffer, 0, &bytes.into_inner());
        let mut bytes = UniformBuffer::new(Vec::<u8>::new());
        bytes
            .write(&scene::cube_model(CUBE_OFFSETS[self.cube_offset]))
            .expect("fits the uniform contract");
        gpu.queue
            .write_buffer(&self.cube_model_buffer, 0, &bytes.into_inner());

        // Pass 1: depth only, from the light; cleared to 1.0 and stored.
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Shadow depth pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &self.shadow_view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(1.0),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..RenderPassDescriptor::default()
            });
            pass.set_pipeline(&self.depth_pipeline);
            pass.set_bind_group(0, &self.cube_shadow_bind_group, &[]);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            // Only the cube casts shadows.
            pass.draw_indexed(CUBE_INDICES, 0, 0..1);
        }

        // Pass 2: only the model bind group differs between floor and cube.
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Shadow mapping main pass"),
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
        pass.set_pipeline(&self.main_pipeline);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        pass.set_bind_group(0, &self.floor_bind_group, &[]);
        pass.draw_indexed(FLOOR_INDICES, 0, 0..1);
        pass.set_bind_group(0, &self.cube_main_bind_group, &[]);
        pass.draw_indexed(CUBE_INDICES, 0, 0..1);
    }

    fn window_event(&mut self, window: &winit::window::Window, event: &WindowEvent) {
        if let WindowEvent::KeyboardInput {
            event: key_event, ..
        } = event
            && key_event.state == ElementState::Pressed
            && let PhysicalKey::Code(key_code) = key_event.physical_key
        {
            // Static frame: redraw only when a key changed it.
            match key_code {
                KeyCode::KeyH => {
                    self.params.shadows_on = 1 - self.params.shadows_on;
                    window.request_redraw();
                }
                KeyCode::KeyM => {
                    self.cube_offset = (self.cube_offset + 1) % CUBE_OFFSETS.len();
                    window.request_redraw();
                }
                _ => {}
            }
        }
    }
}

impl ShadowMapping {
    /// Recreates the local depth attachment for a new target size.
    fn recreate_depth(&mut self, gpu: &Gpu, width: u32, height: u32) {
        let texture = gpu.device.create_texture(&TextureDescriptor {
            label: Some("Shadow mapping depth"),
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
        let view = texture.create_view(&TextureViewDescriptor {
            label: Some("Shadow mapping depth view"),
            ..Default::default()
        });
        self.depth_texture = Some(texture);
        self.depth_view = Some(view);
        self.depth_size = (width, height);
    }

    /// The verification crate drives this directly.
    pub fn set_shadows(&mut self, shadows: bool) {
        self.params.shadows_on = u32::from(shadows);
    }

    /// The verification crate drives this directly.
    pub fn set_cube_offset(&mut self, offset_x: f32) {
        if let Some(index) = CUBE_OFFSETS.iter().position(|&offset| offset == offset_x) {
            self.cube_offset = index;
        }
    }
}

/// Creates a uniform buffer; 64 bytes per mat4x4, 48 for the params.
fn create_uniform(gpu: &Gpu, label: &str, size: u64) -> Buffer {
    gpu.device.create_buffer(&BufferDescriptor {
        label: Some(label),
        size,
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

/// Shared inputs of every main-pass bind group.
struct MainBuffers<'a> {
    layout: &'a BindGroupLayout,
    camera: &'a Buffer,
    light: &'a Buffer,
    params: &'a Buffer,
}

/// Five entries; only the model buffer differs between floor and cube.
fn create_main_bind_group(
    gpu: &Gpu,
    label: &str,
    model: &Buffer,
    shared: &MainBuffers,
    shadow: &TextureView,
) -> BindGroup {
    gpu.device.create_bind_group(&BindGroupDescriptor {
        label: Some(label),
        layout: shared.layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(BufferBinding {
                    buffer: model,
                    offset: 0,
                    size: None,
                }),
            },
            BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Buffer(BufferBinding {
                    buffer: shared.camera,
                    offset: 0,
                    size: None,
                }),
            },
            BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Buffer(BufferBinding {
                    buffer: shared.light,
                    offset: 0,
                    size: None,
                }),
            },
            BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::Buffer(BufferBinding {
                    buffer: shared.params,
                    offset: 0,
                    size: None,
                }),
            },
            BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::TextureView(shadow),
            },
        ],
    })
}
