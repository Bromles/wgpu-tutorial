use std::error::Error;

use encase::UniformBuffer;
use framework::{Gpu, Sample};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, BlendComponent, BlendFactor, BlendOperation, BlendState,
    Buffer, BufferBinding, BufferBindingType, BufferDescriptor, BufferSize, BufferUsages, Color,
    ColorTargetState, ColorWrites, CommandEncoder, CompareFunction, DepthStencilState, Extent3d,
    FragmentState, FrontFace, LoadOp, Operations, PipelineCompilationOptions, PipelineLayout,
    PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, ShaderModule, ShaderStages, StoreOp, Texture, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages, TextureView, TextureViewDescriptor,
    VertexAttribute, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode, include_wgsl,
};
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::Window;

use crate::params::{
    BLACK, CUT_ABOVE, CUT_BELOW, GREEN, HALF_X, HALF_Y, Params, RED, Z_BACKGROUND, Z_SOURCES,
};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    /// Position with w = 1: a point of the design plane at its layer's z.
    position: [f32; 4],
    /// Linear RGBA; the cutout halves carry 0.49 / 0.51 instead of 0.5.
    color: [f32; 4],
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

/// One axis-aligned rectangle from its bounds, depth and constant color.
fn quad(x0: f32, y0: f32, x1: f32, y1: f32, z: f32, color: [f32; 4]) -> [Vertex; 4] {
    [
        Vertex {
            position: [x0, y0, z, 1.0],
            color,
        },
        Vertex {
            position: [x1, y0, z, 1.0],
            color,
        },
        Vertex {
            position: [x0, y1, z, 1.0],
            color,
        },
        Vertex {
            position: [x1, y1, z, 1.0],
            color,
        },
    ]
}

/// Preset B of 27a: the whole frame is the opaque black background.
fn background_vertices() -> [Vertex; 4] {
    quad(-HALF_X, -HALF_Y, HALF_X, HALF_Y, Z_BACKGROUND, BLACK)
}

fn blend_vertices() -> [Vertex; 8] {
    let red = quad(-1.5, -0.9, -0.3, 0.9, Z_SOURCES, RED);
    let green = quad(-0.9, -0.9, 0.3, 0.9, Z_SOURCES, GREEN);
    let mut vertices = [Vertex {
        position: [0.0; 4],
        color: [0.0; 4],
    }; 8];
    for (part, chunk) in [red, green]
        .iter()
        .zip(vertices.as_chunks_mut::<4>().0.iter_mut())
    {
        chunk.copy_from_slice(part);
    }
    vertices
}

/// Each source splits into halves with alpha just below/above the threshold.
fn cutout_vertices() -> [Vertex; 16] {
    let quads = [
        quad(
            -1.5,
            0.0,
            -0.3,
            0.9,
            Z_SOURCES,
            [RED[0], RED[1], RED[2], CUT_BELOW],
        ),
        quad(
            -1.5,
            -0.9,
            -0.3,
            0.0,
            Z_SOURCES,
            [RED[0], RED[1], RED[2], CUT_ABOVE],
        ),
        quad(
            -0.9,
            0.0,
            0.3,
            0.9,
            Z_SOURCES,
            [GREEN[0], GREEN[1], GREEN[2], CUT_BELOW],
        ),
        quad(
            -0.9,
            -0.9,
            0.3,
            0.0,
            Z_SOURCES,
            [GREEN[0], GREEN[1], GREEN[2], CUT_ABOVE],
        ),
    ];
    let mut vertices = [Vertex {
        position: [0.0; 4],
        color: [0.0; 4],
    }; 16];
    for (part, chunk) in quads.iter().zip(vertices.as_chunks_mut::<4>().0.iter_mut()) {
        chunk.copy_from_slice(part);
    }
    vertices
}

/// Six indices per quad; bases restart from zero per vertex buffer.
fn indices() -> [u16; 42] {
    const BASES: [u16; 7] = [0, 0, 4, 0, 4, 8, 12];
    let mut indices = [0u16; 42];
    for (q, block) in indices.as_chunks_mut::<6>().0.iter_mut().enumerate() {
        let base = BASES[q];
        block.copy_from_slice(&[base, base + 1, base + 2, base + 2, base + 1, base + 3]);
    }
    indices
}

/// Index blocks of the three vertex buffers, in buffer order.
const BACKGROUND: std::ops::Range<u32> = 0..6;
const BLEND_RED: std::ops::Range<u32> = 6..12;
const BLEND_GREEN: std::ops::Range<u32> = 12..18;
const CUT_RED_UPPER: std::ops::Range<u32> = 18..24;
const CUT_RED_LOWER: std::ops::Range<u32> = 24..30;
const CUT_GREEN_UPPER: std::ops::Range<u32> = 30..36;
const CUT_GREEN_LOWER: std::ops::Range<u32> = 36..42;

/// Over for straight sources, as in chapter 27a.
const STRAIGHT: BlendState = BlendState {
    color: BlendComponent {
        src_factor: BlendFactor::SrcAlpha,
        dst_factor: BlendFactor::OneMinusSrcAlpha,
        operation: BlendOperation::Add,
    },
    alpha: BlendComponent {
        src_factor: BlendFactor::One,
        dst_factor: BlendFactor::OneMinusSrcAlpha,
        operation: BlendOperation::Add,
    },
};

/// Over for premultiplied sources, as in chapter 27a.
const PREMULTIPLIED: BlendState = BlendState {
    color: BlendComponent {
        src_factor: BlendFactor::One,
        dst_factor: BlendFactor::OneMinusSrcAlpha,
        operation: BlendOperation::Add,
    },
    alpha: STRAIGHT.alpha,
};

/// Draw order of the two sources; the O key swaps it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Order {
    RedFirst,
    GreenFirst,
}

/// Chapter 27b: preset B of 27a plus a depth buffer and a cutout mode.
/// Opaque writes depth; transparent sources only test it.
pub struct BlendOrder {
    opaque_pipeline: RenderPipeline,
    straight_pipeline: RenderPipeline,
    premultiplied_pipeline: RenderPipeline,
    cutout_pipeline: RenderPipeline,
    bind_group: BindGroup,
    background_buffer: Buffer,
    blend_buffer: Buffer,
    cutout_buffer: Buffer,
    index_buffer: Buffer,
    premultiplied: bool,
    cutout: bool,
    order: Order,
    depth_texture: Option<Texture>,
    depth_view: Option<TextureView>,
    depth_size: (u32, u32),
    /// Nonzero target size seen in the events; consumed by the next draw.
    pending_size: Option<PhysicalSize<u32>>,
}

impl Sample for BlendOrder {
    fn init(gpu: &Gpu) -> Result<Self, Box<dyn Error>> {
        let background = background_vertices();
        let blend = blend_vertices();
        let cutout = cutout_vertices();
        let indices = indices();
        let shader = gpu
            .device
            .create_shader_module(include_wgsl!("shader.wgsl"));
        let layout = gpu
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Blend order layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(64),
                    },
                    count: None,
                }],
            });
        let pipeline_layout = gpu
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Blend order pipeline layout"),
                bind_group_layouts: &[Some(&layout)],
                immediate_size: 0,
            });
        // Four pipelines: opaque and cutout write depth, transparent ones only test.
        let opaque_pipeline = create_pipeline(
            gpu,
            &pipeline_layout,
            &shader,
            "Blend order pipeline (opaque)",
            "fs_straight",
            None,
            true,
        );
        let straight_pipeline = create_pipeline(
            gpu,
            &pipeline_layout,
            &shader,
            "Blend order pipeline (straight)",
            "fs_straight",
            Some(STRAIGHT),
            false,
        );
        let premultiplied_pipeline = create_pipeline(
            gpu,
            &pipeline_layout,
            &shader,
            "Blend order pipeline (premultiplied)",
            "fs_premultiplied",
            Some(PREMULTIPLIED),
            false,
        );
        let cutout_pipeline = create_pipeline(
            gpu,
            &pipeline_layout,
            &shader,
            "Blend order pipeline (cutout)",
            "fs_cutout",
            None,
            true,
        );
        let params_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Blend order params"),
            size: 64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        // The camera never moves: the uniform is written once, not per frame.
        let mut bytes = UniformBuffer::new(Vec::<u8>::new());
        bytes
            .write(&Params {
                view_proj: crate::params::ortho(),
            })
            .expect("fits the uniform contract");
        gpu.queue
            .write_buffer(&params_buffer, 0, &bytes.into_inner());
        let bind_group = gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Blend order bind group"),
            layout: &layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(BufferBinding {
                    buffer: &params_buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });
        let background_buffer = create_vertex_buffer(gpu, "Background quad", &background);
        let blend_buffer = create_vertex_buffer(gpu, "Blend source quads", &blend);
        let cutout_buffer = create_vertex_buffer(gpu, "Cutout half quads", &cutout);
        let index_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Blend order indices"),
            size: size_of_val(&indices) as u64,
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&index_buffer, 0, bytemuck::cast_slice(&indices));
        Ok(Self {
            opaque_pipeline,
            straight_pipeline,
            premultiplied_pipeline,
            cutout_pipeline,
            bind_group,
            background_buffer,
            blend_buffer,
            cutout_buffer,
            index_buffer,
            premultiplied: false,
            cutout: false,
            order: Order::RedFirst,
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

        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Blend order pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color {
                        r: 0.08,
                        g: 0.08,
                        b: 0.1,
                        a: 1.0,
                    }),
                    store: StoreOp::Store,
                },
                depth_slice: None,
            })],
            // Clear depth to 1.0: the "farther than everything" wall under `Less`.
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
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

        // Opaque first: the background writes color and depth (0.75).
        pass.set_pipeline(&self.opaque_pipeline);
        pass.set_vertex_buffer(0, self.background_buffer.slice(..));
        pass.draw_indexed(BACKGROUND, 0, 0..1);

        if self.cutout {
            // Passing cutout fragments write depth; the first draw keeps the overlap under `Less`.
            pass.set_pipeline(&self.cutout_pipeline);
            pass.set_vertex_buffer(0, self.cutout_buffer.slice(..));
            match self.order {
                Order::RedFirst => {
                    pass.draw_indexed(CUT_RED_UPPER, 0, 0..1);
                    pass.draw_indexed(CUT_RED_LOWER, 0, 0..1);
                    pass.draw_indexed(CUT_GREEN_UPPER, 0, 0..1);
                    pass.draw_indexed(CUT_GREEN_LOWER, 0, 0..1);
                }
                Order::GreenFirst => {
                    pass.draw_indexed(CUT_GREEN_UPPER, 0, 0..1);
                    pass.draw_indexed(CUT_GREEN_LOWER, 0, 0..1);
                    pass.draw_indexed(CUT_RED_UPPER, 0, 0..1);
                    pass.draw_indexed(CUT_RED_LOWER, 0, 0..1);
                }
            }
        } else {
            // Transparent after opaque: depth test on, write off.
            let sources = if self.premultiplied {
                &self.premultiplied_pipeline
            } else {
                &self.straight_pipeline
            };
            pass.set_pipeline(sources);
            pass.set_vertex_buffer(0, self.blend_buffer.slice(..));
            match self.order {
                Order::RedFirst => {
                    pass.draw_indexed(BLEND_RED, 0, 0..1);
                    pass.draw_indexed(BLEND_GREEN, 0, 0..1);
                }
                Order::GreenFirst => {
                    pass.draw_indexed(BLEND_GREEN, 0, 0..1);
                    pass.draw_indexed(BLEND_RED, 0, 0..1);
                }
            }
        }
    }

    fn window_event(&mut self, window: &Window, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput {
                event: key_event, ..
            } if key_event.state == ElementState::Pressed
                && let PhysicalKey::Code(key_code) = key_event.physical_key =>
            {
                // The frame is static: repaint exactly when a key changed it.
                match key_code {
                    KeyCode::KeyM => {
                        self.premultiplied = !self.premultiplied;
                        window.request_redraw();
                    }
                    KeyCode::KeyO => {
                        self.order = match self.order {
                            Order::RedFirst => Order::GreenFirst,
                            Order::GreenFirst => Order::RedFirst,
                        };
                        window.request_redraw();
                    }
                    KeyCode::KeyT => {
                        self.cutout = !self.cutout;
                        window.request_redraw();
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

impl BlendOrder {
    /// Recreates the local depth attachment for a new target size.
    fn recreate_depth(&mut self, gpu: &Gpu, width: u32, height: u32) {
        let texture = gpu.device.create_texture(&TextureDescriptor {
            label: Some("Blend order depth"),
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

    /// Selects the blend-mode source representation; driven by the verification crate.
    pub fn set_representation(&mut self, premultiplied: bool) {
        self.premultiplied = premultiplied;
    }

    /// Sets the draw order of the two sources; driven by the verification crate.
    pub fn set_order(&mut self, order: Order) {
        self.order = order;
    }

    /// Blended sources vs the cutout pattern; driven by the verification crate.
    pub fn set_cutout(&mut self, cutout: bool) {
        self.cutout = cutout;
    }
}

fn create_vertex_buffer(gpu: &Gpu, label: &str, vertices: &[Vertex]) -> Buffer {
    let buffer = gpu.device.create_buffer(&BufferDescriptor {
        label: Some(label),
        size: size_of_val(vertices) as u64,
        usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    gpu.queue
        .write_buffer(&buffer, 0, bytemuck::cast_slice(vertices));
    buffer
}

fn create_pipeline(
    gpu: &Gpu,
    layout: &PipelineLayout,
    shader: &ShaderModule,
    label: &str,
    fragment_entry: &str,
    blend: Option<BlendState>,
    depth_write: bool,
) -> RenderPipeline {
    gpu.device
        .create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(label),
            layout: Some(layout),
            vertex: VertexState {
                module: shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(Vertex::LAYOUT)],
                compilation_options: PipelineCompilationOptions::default(),
            },
            fragment: Some(FragmentState {
                module: shader,
                entry_point: Some(fragment_entry),
                targets: &[Some(ColorTargetState {
                    format: gpu.format,
                    blend,
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
            // Opaque and cutout write depth, transparent only tests; comparison stays `Less`.
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: Some(depth_write),
                depth_compare: Some(CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            cache: None,
            multiview_mask: None,
        })
}
