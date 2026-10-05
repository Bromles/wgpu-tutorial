use std::error::Error;

use framework::{Gpu, Sample};
use crate::mesh::{Params,Vertex,build_strip_mesh};
use wgpu::{BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBinding, BufferBindingType,
    BufferDescriptor, BufferSize, BufferUsages, Color, ColorTargetState, ColorWrites,
    CommandEncoder, ComputePassDescriptor, ComputePipeline, ComputePipelineDescriptor,
    FragmentState, FrontFace, LoadOp, Operations, PipelineCompilationOptions,
    PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology, RenderPassColorAttachment,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, ShaderStages, StoreOp,
    TextureView, VertexState, include_wgsl, };
use winit::event::{ElementState,WindowEvent};
use winit::keyboard::{KeyCode,PhysicalKey};
use winit::window::Window;
use wgpu::MultisampleState;
use wgpu::BindingResource;
use bytemuck::cast_slice;
use bytemuck::bytes_of;
use wgpu::IndexFormat;

/// One slot per indicator cell; value i mapsto i / 256.
pub const CELLS: u32 = 257;
/// Invocations per workgroup; launches round up to whole groups.
pub const WORKGROUP_SIZE: u32 = 64;

/// A compute pass fills the values; the render pass reads them via storage.
pub struct ComputeBasics {
    fill_pipeline: ComputePipeline,
    strip_pipeline: RenderPipeline,
    fill_bind_group: BindGroup,
    strip_bind_group: BindGroup,
    params_buffer: Buffer,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    count: u32,
}

impl Sample for ComputeBasics {
    fn init(gpu: &Gpu) -> Result<Self, Box<dyn Error>> {
        // Requires storage buffers in both compute and vertex stages.
        if gpu.device.limits().max_storage_buffers_per_shader_stage < 1 {
            return Err(
                "This example requires at least one storage buffer per shader stage".into(),
            );
        }

        let fill_shader = gpu.device.create_shader_module(include_wgsl!("fill.wgsl"));
        let strip_shader = gpu.device.create_shader_module(include_wgsl!("strip.wgsl"));
        let fill_layout = gpu
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Fill bind group layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(4),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(4),
                        },
                        count: None,
                    },
                ],
            });
        let strip_layout = gpu
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Strip bind group layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(4),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::VERTEX,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(4),
                        },
                        count: None,
                    },
                ],
            });
        let fill_pipeline_layout = gpu
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Fill pipeline layout"),
                bind_group_layouts: &[Some(&fill_layout)],
                immediate_size: 0,
            });
        let strip_pipeline_layout = gpu
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Strip pipeline layout"),
                bind_group_layouts: &[Some(&strip_layout)],
                immediate_size: 0,
            });
        let fill_pipeline = gpu
            .device
            .create_compute_pipeline(&ComputePipelineDescriptor {
                label: Some("Fill values pipeline"),
                layout: Some(&fill_pipeline_layout),
                module: &fill_shader,
                entry_point: Some("fill"),
                compilation_options: PipelineCompilationOptions::default(),
                cache: None,
            });
        let strip_pipeline = gpu
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Indicator strip pipeline"),
                layout: Some(&strip_pipeline_layout),
                vertex: VertexState {
                    module: &strip_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[Some(Vertex::LAYOUT)],
                    compilation_options: PipelineCompilationOptions::default(),
                },
                fragment: Some(FragmentState {
                    module: &strip_shader,
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

        // GPU-only: compute writes it, vertex reads it, CPU never does.
        let values_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Strip values"),
            size: u64::from(CELLS) * 4,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let params_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Strip params"),
            size: 4,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let fill_bind_group = gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Fill bind group"),
            layout: &fill_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &values_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &params_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });
        let strip_bind_group = gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Strip bind group"),
            layout: &strip_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &values_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &params_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });

        let (vertices, indices) = build_strip_mesh();
        let vertex_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Strip mesh"),
            size: (vertices.len() * size_of::<Vertex>()) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&vertex_buffer, 0, cast_slice(&vertices));
        let index_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Strip indices"),
            size: (indices.len() * size_of::<u16>()) as u64,
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&index_buffer, 0, cast_slice(&indices));
        Ok(Self {
            fill_pipeline,
            strip_pipeline,
            fill_bind_group,
            strip_bind_group,
            params_buffer,
            vertex_buffer,
            index_buffer,
            count: CELLS,
        })
    }

    fn draw(&mut self,
    gpu: &Gpu,
    encoder: &mut CommandEncoder,
    view: &TextureView) {
        let params = Params { count: self.count };
        gpu.queue
            .write_buffer(&self.params_buffer, 0, bytes_of(&params));

        // Pass 1: fill the values.
        {
            let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("Fill values pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.fill_pipeline);
            pass.set_bind_group(0, &self.fill_bind_group, &[]);
            let groups = self.count.div_ceil(WORKGROUP_SIZE);
            pass.dispatch_workgroups(groups, 1, 1);
        }

        // Pass 2: draw the strip from what pass 1 wrote.
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Indicator strip pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color {
                        r: 0.08,
                        g: 0.08,
                        b: 0.12,
                        a: 1.0,
                    }),
                    store: StoreOp::Store,
                },
                depth_slice: None,
            })],
            ..RenderPassDescriptor::default()
        });
        pass.set_pipeline(&self.strip_pipeline);
        pass.set_bind_group(0, &self.strip_bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint16);
        pass.draw_indexed(0..CELLS * 6, 0, 0..1);
    }

    fn window_event(&mut self,
    window: &Window,
    event: &WindowEvent) {
        if let WindowEvent::KeyboardInput {
            event: key_event, ..
        } = event
            && key_event.state == ElementState::Pressed
            && let PhysicalKey::Code(key_code) = key_event.physical_key
            && key_code == KeyCode::KeyR
        {
            // Toggle between 257 and 256 filled cells.
            self.count = if self.count == CELLS { 256 } else { CELLS };
            window.request_redraw();
        }
    }
}

impl ComputeBasics {
    /// Clamp keeps the shader denominator positive.
    pub fn set_count(&mut self,
    count: u32) {
        self.count = count.clamp(2, CELLS);
    }
}
