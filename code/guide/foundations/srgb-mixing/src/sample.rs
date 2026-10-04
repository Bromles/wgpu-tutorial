use encase::UniformBuffer;
use framework::{Gpu, Sample};
use std::error::Error;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBinding, BufferBindingType, BufferDescriptor,
    BufferSize, BufferUsages, Color, ColorTargetState, ColorWrites, CommandEncoder, FragmentState,
    FrontFace, LoadOp, Operations, PipelineCompilationOptions, PipelineLayoutDescriptor,
    PrimitiveState, PrimitiveTopology, RenderPassColorAttachment, RenderPassDescriptor,
    RenderPipeline, RenderPipelineDescriptor, ShaderStages, StoreOp, TextureView, VertexAttribute,
    VertexBufferLayout, VertexFormat, VertexState, VertexStepMode, include_wgsl,
};

use crate::color::{mean_linear, mean_of_codes, quantize_u8, srgb_decode, srgb_encode};
use glam::Vec4;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 4],
    color: [f32; 4],
}

/// Two side-by-side quads; the tone comes from the per-draw uniform, not vertex colors.
const VERTICES: [Vertex; 8] = [
    // Left quad: x in [-0.75, 0.0].
    Vertex {
        position: [-0.75, 0.75, 0.5, 1.0],
        color: [1.0, 1.0, 1.0, 1.0],
    },
    Vertex {
        position: [0.0, 0.75, 0.5, 1.0],
        color: [1.0, 1.0, 1.0, 1.0],
    },
    Vertex {
        position: [-0.75, -0.75, 0.5, 1.0],
        color: [1.0, 1.0, 1.0, 1.0],
    },
    Vertex {
        position: [0.0, -0.75, 0.5, 1.0],
        color: [1.0, 1.0, 1.0, 1.0],
    },
    // Right quad: x in [0.0, 0.75].
    Vertex {
        position: [0.0, 0.75, 0.5, 1.0],
        color: [1.0, 1.0, 1.0, 1.0],
    },
    Vertex {
        position: [0.75, 0.75, 0.5, 1.0],
        color: [1.0, 1.0, 1.0, 1.0],
    },
    Vertex {
        position: [0.0, -0.75, 0.5, 1.0],
        color: [1.0, 1.0, 1.0, 1.0],
    },
    Vertex {
        position: [0.75, -0.75, 0.5, 1.0],
        color: [1.0, 1.0, 1.0, 1.0],
    },
];

const INDICES: [u16; 12] = [0, 1, 2, 2, 1, 3, 4, 5, 6, 6, 5, 7];

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

/// Per-side tone, computed on the CPU with the real transfer function.
#[derive(encase::ShaderType, Debug, Clone, Copy)]
struct Tone {
    color: Vec4,
}

/// Chapter 12: correct and erroneous black/white mixing side by side.
pub struct SrgbMixing {
    pipeline: RenderPipeline,
    bind_groups: [BindGroup; 2],
    vertex_buffer: Buffer,
    index_buffer: Buffer,
}

impl Sample for SrgbMixing {
    fn init(gpu: &Gpu) -> Result<Self, Box<dyn Error>> {
        let shader = gpu
            .device
            .create_shader_module(include_wgsl!("shader.wgsl"));
        let tone_layout = gpu
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Tone bind group layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(16),
                    },
                    count: None,
                }],
            });
        let pipeline_layout = gpu
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Tone pipeline layout"),
                bind_group_layouts: &[Some(&tone_layout)],
                immediate_size: 0,
            });
        let pipeline = gpu
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("sRGB mixing pipeline"),
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
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                cache: None,
                multiview_mask: None,
            });

        // Left: mix light -> 188. Right: mix codes -> stays near 128.
        let correct_linear = mean_linear(0.0, 1.0);
        let wrong_code =
            mean_of_codes(quantize_u8(srgb_encode(0.0)), quantize_u8(srgb_encode(1.0)));
        let wrong_linear = srgb_decode(f32::from(wrong_code) / 255.0);
        tracing::info!(
            correct_code = quantize_u8(srgb_encode(correct_linear)),
            wrong_code,
            wrong_linear,
            "Mixing comparison values"
        );
        let tones = [
            Tone {
                color: Vec4::new(correct_linear, correct_linear, correct_linear, 1.0),
            },
            Tone {
                color: Vec4::new(wrong_linear, wrong_linear, wrong_linear, 1.0),
            },
        ];

        let buffers = tones.each_ref().map(|tone| {
            let buffer = gpu.device.create_buffer(&BufferDescriptor {
                label: Some("Tone uniform buffer"),
                size: 16,
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let mut bytes = UniformBuffer::new(Vec::<u8>::new());
            bytes.write(tone).expect("fits the uniform contract");
            gpu.queue.write_buffer(&buffer, 0, &bytes.into_inner());
            buffer
        });
        let bind_groups = buffers.each_ref().map(|buffer| {
            gpu.device.create_bind_group(&BindGroupDescriptor {
                label: Some("Tone bind group"),
                layout: &tone_layout,
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(BufferBinding {
                        buffer,
                        offset: 0,
                        size: None,
                    }),
                }],
            })
        });

        let vertex_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Quad vertices"),
            size: size_of_val(&VERTICES) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&VERTICES));
        let index_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Quad indices"),
            size: size_of_val(&INDICES) as u64,
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&index_buffer, 0, bytemuck::cast_slice(&INDICES));
        Ok(Self {
            pipeline,
            bind_groups,
            vertex_buffer,
            index_buffer,
        })
    }

    fn draw(&mut self, _gpu: &Gpu, encoder: &mut CommandEncoder, view: &TextureView) {
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("sRGB mixing pass"),
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
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        // Left quad: the correct mix of light.
        pass.set_bind_group(0, &self.bind_groups[0], &[]);
        pass.draw_indexed(0..6, 0, 0..1);
        // Right quad: what the mean of codes actually represents.
        pass.set_bind_group(0, &self.bind_groups[1], &[]);
        pass.draw_indexed(6..12, 0, 0..1);
    }
}
