use bytemuck::{Pod, Zeroable, cast_slice};
use framework::{Gpu, Sample};
use std::error::Error;
use wgpu::{
    Buffer, BufferAddress, BufferDescriptor, BufferUsages, Color, ColorTargetState, ColorWrites,
    CommandEncoder, FragmentState, FrontFace, IndexFormat, LoadOp, MultisampleState, Operations,
    PipelineCompilationOptions, PrimitiveState, PrimitiveTopology, RenderPassColorAttachment,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, StoreOp, TextureView,
    VertexAttribute, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode, include_wgsl,
};

/// Same 32-byte record as in chapter 07a.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    position: [f32; 4],
    color: [f32; 4],
}

// The four pixels of the chapter 01 image as rectangle corners.
const VERTICES: [Vertex; 4] = [
    Vertex {
        position: [-0.75, 0.75, 0.5, 1.0],
        color: [1.0, 0.0, 0.0, 1.0],
    },
    Vertex {
        position: [0.75, 0.75, 0.5, 1.0],
        color: [0.0, 1.0, 0.0, 1.0],
    },
    Vertex {
        position: [-0.75, -0.75, 0.5, 1.0],
        color: [0.0, 0.0, 1.0, 1.0],
    },
    Vertex {
        position: [0.75, -0.75, 0.5, 1.0],
        color: [1.0, 1.0, 1.0, 1.0],
    },
];

// Indices reuse the shared BL-TR diagonal corners instead of duplicating records.
const INDICES: [u16; 6] = [0, 1, 2, 2, 1, 3];

impl Vertex {
    const LAYOUT: VertexBufferLayout<'static> = VertexBufferLayout {
        array_stride: size_of::<Vertex>() as BufferAddress,
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

/// Chapter 07b: an indexed rectangle built from four records and six u16.
pub struct IndexedGeometry {
    pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
}

impl Sample for IndexedGeometry {
    fn init(gpu: &Gpu) -> Result<Self, Box<dyn Error>> {
        let shader = gpu
            .device
            .create_shader_module(include_wgsl!("shader.wgsl"));
        let pipeline = gpu
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Indexed geometry pipeline"),
                layout: None,
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
                depth_stencil: None,
                multisample: MultisampleState::default(),
                cache: None,
                multiview_mask: None,
            });
        let vertex_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Rectangle vertices"),
            size: size_of_val(&VERTICES) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&vertex_buffer, 0, cast_slice(&VERTICES));
        let index_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Rectangle indices"),
            size: size_of_val(&INDICES) as u64,
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&index_buffer, 0, cast_slice(&INDICES));
        Ok(Self {
            pipeline,
            vertex_buffer,
            index_buffer,
        })
    }

    fn draw(&mut self,
    _gpu: &Gpu,
    encoder: &mut CommandEncoder,
    view: &TextureView) {
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Indexed geometry pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color {
                        r: 0.5,
                        g: 0.5,
                        b: 0.5,
                        a: 1.0,
                    }),
                    store: StoreOp::Store,
                },
                depth_slice: None,
            })],
            ..RenderPassDescriptor::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint16);
        pass.draw_indexed(0..6, 0, 0..1);
    }
}
