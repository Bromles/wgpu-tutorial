use std::error::Error;

use encase::UniformBuffer;
use framework::{Gpu, Sample};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, BlendComponent, BlendFactor, BlendOperation, BlendState,
    Buffer, BufferBinding, BufferBindingType, BufferDescriptor, BufferSize, BufferUsages, Color,
    ColorTargetState, ColorWrites, CommandEncoder, FragmentState, FrontFace, LoadOp, Operations,
    PipelineCompilationOptions, PipelineLayout, PipelineLayoutDescriptor, PrimitiveState,
    PrimitiveTopology, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, ShaderModule, ShaderStages, StoreOp, TextureView, VertexAttribute,
    VertexBufferLayout, VertexFormat, VertexState, VertexStepMode, include_wgsl,
};
use winit::event::{ElementState, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

use crate::params::{BLACK, BLUE, GREEN, HALF_X, HALF_Y, Params, RED, Z};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    /// Position with w = 1: a point of the z = Z plane.
    position: [f32; 4],
    /// Linear RGBA; opaque quads carry a = 1, sources a = 0.5.
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

/// One axis-aligned rectangle of the design plane.
fn quad(x0: f32, y0: f32, x1: f32, y1: f32, color: [f32; 4]) -> [Vertex; 4] {
    [
        Vertex {
            position: [x0, y0, Z, 1.0],
            color,
        },
        Vertex {
            position: [x1, y0, Z, 1.0],
            color,
        },
        Vertex {
            position: [x0, y1, Z, 1.0],
            color,
        },
        Vertex {
            position: [x1, y1, Z, 1.0],
            color,
        },
    ]
}

/// Backgrounds first, then sources; preset A left, preset B right.
fn vertices() -> [Vertex; 20] {
    let quads = [
        quad(-HALF_X, -HALF_Y, 0.0, HALF_Y, BLUE),
        quad(0.0, -HALF_Y, HALF_X, HALF_Y, BLACK),
        quad(-1.8, -0.9, -0.6, 0.9, RED),
        quad(0.3, -0.9, 1.5, 0.9, RED),
        quad(0.9, -0.9, 2.1, 0.9, GREEN),
    ];
    let mut vertices = [Vertex {
        position: [0.0; 4],
        color: [0.0; 4],
    }; 20];
    for (quad, chunk) in quads.iter().zip(vertices.as_chunks_mut::<4>().0.iter_mut()) {
        chunk.copy_from_slice(quad);
    }
    vertices
}

/// Six indices per quad, each block addressing its own four vertices.
fn indices() -> [u16; 30] {
    let mut indices = [0u16; 30];
    for (q, block) in indices.as_chunks_mut::<6>().0.iter_mut().enumerate() {
        let base = (q * 4) as u16;
        block.copy_from_slice(&[base, base + 1, base + 2, base + 2, base + 1, base + 3]);
    }
    indices
}

/// Index ranges of the two passes: backgrounds (opaque) and sources.
const BACKGROUNDS: std::ops::Range<u32> = 0..12;
const SOURCES: std::ops::Range<u32> = 12..30;

/// Over for straight sources; components written out field by field.
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

/// Over for premultiplied sources; the source factor is simply One.
const PREMULTIPLIED: BlendState = BlendState {
    color: BlendComponent {
        src_factor: BlendFactor::One,
        dst_factor: BlendFactor::OneMinusSrcAlpha,
        operation: BlendOperation::Add,
    },
    alpha: STRAIGHT.alpha,
};

/// Chapter 27a: two diagnostic presets composed with over.
/// M switches straight/premultiplied; the frame must not change.
pub struct BlendOver {
    opaque_pipeline: RenderPipeline,
    straight_pipeline: RenderPipeline,
    premultiplied_pipeline: RenderPipeline,
    bind_group: BindGroup,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    premultiplied: bool,
}

impl Sample for BlendOver {
    fn init(gpu: &Gpu) -> Result<Self, Box<dyn Error>> {
        let vertices = vertices();
        let indices = indices();
        let shader = gpu
            .device
            .create_shader_module(include_wgsl!("shader.wgsl"));
        let layout = gpu
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Blend over layout"),
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
                label: Some("Blend over pipeline layout"),
                bind_group_layouts: &[Some(&layout)],
                immediate_size: 0,
            });
        // One geometry, three pipelines: opaque, straight, premultiplied.
        let opaque_pipeline = create_pipeline(
            gpu,
            &pipeline_layout,
            &shader,
            "Blend over pipeline (opaque)",
            "fs_straight",
            None,
        );
        let straight_pipeline = create_pipeline(
            gpu,
            &pipeline_layout,
            &shader,
            "Blend over pipeline (straight)",
            "fs_straight",
            Some(STRAIGHT),
        );
        let premultiplied_pipeline = create_pipeline(
            gpu,
            &pipeline_layout,
            &shader,
            "Blend over pipeline (premultiplied)",
            "fs_premultiplied",
            Some(PREMULTIPLIED),
        );
        let params_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Blend over params"),
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
            label: Some("Blend over bind group"),
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
        let vertex_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Preset quads"),
            size: size_of_val(&vertices) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        let index_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Preset quad indices"),
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
            bind_group,
            vertex_buffer,
            index_buffer,
            premultiplied: false,
        })
    }

    fn draw(&mut self, _gpu: &Gpu, encoder: &mut CommandEncoder, view: &TextureView) {
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Blend over pass"),
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
            ..RenderPassDescriptor::default()
        });
        pass.set_pipeline(&self.opaque_pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        // Opaque first: the backgrounds, alpha = 1, no blending.
        pass.draw_indexed(BACKGROUNDS, 0, 0..1);
        // Sources after opaque, in a fixed order.
        let sources = if self.premultiplied {
            &self.premultiplied_pipeline
        } else {
            &self.straight_pipeline
        };
        pass.set_pipeline(sources);
        pass.draw_indexed(SOURCES, 0, 0..1);
    }

    fn window_event(&mut self, window: &winit::window::Window, event: &WindowEvent) {
        if let WindowEvent::KeyboardInput {
            event: key_event, ..
        } = event
            && key_event.state == ElementState::Pressed
            && let PhysicalKey::Code(KeyCode::KeyM) = key_event.physical_key
        {
            // Same operator, different representation: the frame must not change.
            self.premultiplied = !self.premultiplied;
            window.request_redraw();
        }
    }
}

impl BlendOver {
    /// Selects the source representation; driven by the verification crate.
    pub fn set_representation(&mut self, premultiplied: bool) {
        self.premultiplied = premultiplied;
    }
}

fn create_pipeline(
    gpu: &Gpu,
    layout: &PipelineLayout,
    shader: &ShaderModule,
    label: &str,
    fragment_entry: &str,
    blend: Option<BlendState>,
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
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            cache: None,
            multiview_mask: None,
        })
}
