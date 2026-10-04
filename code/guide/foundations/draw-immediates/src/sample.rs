use encase::UniformBuffer;
use framework::{Gpu, Sample};
use std::error::Error;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferAddress, BufferBinding, BufferBindingType,
    BufferDescriptor, BufferSize, BufferUsages, Color, ColorTargetState, ColorWrites,
    CommandEncoder, FragmentState, FrontFace, LoadOp, Operations, PipelineCompilationOptions,
    PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology, RenderPassColorAttachment,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, ShaderStages, StoreOp,
    TextureView, VertexAttribute, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
    include_wgsl,
};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 4],
    color: [f32; 4],
}

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

const INDICES: [u16; 6] = [0, 1, 2, 2, 1, 3];

// A neutral tint for the first draw and a greenish one for the second.
const TINTS: [[f32; 4]; 2] = [[1.0, 1.0, 1.0, 1.0], [0.1, 1.0, 0.3, 1.0]];

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

/// Chapter 11: per-draw tint selection via immediates; gain stays uniform.
pub struct DrawImmediates {
    pipeline: RenderPipeline,
    params_bind_group: BindGroup,
    tints_bind_group: BindGroup,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    selected: [u32; 2],
}

impl Sample for DrawImmediates {
    fn init(gpu: &Gpu) -> Result<Self, Box<dyn Error>> {
        if !gpu.device.features().contains(wgpu::Features::IMMEDIATES) {
            return Err(
                "This example requires the IMMEDIATES feature; run the 09 binding path instead"
                    .into(),
            );
        }
        if gpu.device.limits().max_immediate_size < 4 {
            return Err("This example requires max_immediate_size >= 4".into());
        }
        let shader = gpu
            .device
            .create_shader_module(include_wgsl!("shader.wgsl"));
        let params_layout = gpu
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Params bind group layout"),
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
        let tints_layout = gpu
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Tints bind group layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let pipeline_layout = gpu
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Immediates pipeline layout"),
                bind_group_layouts: &[Some(&params_layout), Some(&tints_layout)],
                // The immediate block of this pipeline: one u32.
                immediate_size: 4,
            });
        let pipeline = gpu
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Immediates pipeline"),
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

        let params_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Params uniform buffer"),
            size: 16,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut params_bytes = UniformBuffer::new(Vec::<u8>::new());
        #[derive(encase::ShaderType)]
        struct Params {
            gain: f32,
        }
        params_bytes
            .write(&Params { gain: 1.0 })
            .expect("fits the uniform contract");
        gpu.queue
            .write_buffer(&params_buffer, 0, &params_bytes.into_inner());
        let params_bind_group = gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Params bind group"),
            layout: &params_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(BufferBinding {
                    buffer: &params_buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        let tints_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Tints storage"),
            size: size_of_val(&TINTS) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&tints_buffer, 0, bytemuck::cast_slice(&TINTS));
        let tints_bind_group = gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Tints bind group"),
            layout: &tints_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(BufferBinding {
                    buffer: &tints_buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        let vertex_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Rectangle vertices"),
            size: size_of_val(&VERTICES) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&VERTICES));
        let index_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Rectangle indices"),
            size: size_of_val(&INDICES) as u64,
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&index_buffer, 0, bytemuck::cast_slice(&INDICES));
        Ok(Self {
            pipeline,
            params_bind_group,
            tints_bind_group,
            vertex_buffer,
            index_buffer,
            selected: [0, 1],
        })
    }

    fn draw(&mut self, _gpu: &Gpu, encoder: &mut CommandEncoder, view: &TextureView) {
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Immediates pass"),
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
        pass.set_bind_group(0, &self.params_bind_group, &[]);
        pass.set_bind_group(1, &self.tints_bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        // The index travels in command state: no upload between draws.
        pass.set_immediates(0, bytemuck::bytes_of(&self.selected[0]));
        pass.draw_indexed(0..3, 0, 0..1);
        pass.set_immediates(0, bytemuck::bytes_of(&self.selected[1]));
        pass.draw_indexed(3..6, 0, 0..1);
    }
}

impl DrawImmediates {
    /// The verification crate checks the documented default: with both
    /// values zeroed, both draws read index 0 and paint the natural colors.
    pub fn drop_second_selection(&mut self) {
        self.selected = [0, 0];
    }
}
