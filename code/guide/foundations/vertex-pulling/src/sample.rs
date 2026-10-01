use encase::UniformBuffer;
use shell::{Gpu, Sample};
use std::error::Error;
use std::time::Instant;
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
use winit::event::{ElementState, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

use crate::params::Params;

/// Gain growth speed, per second; gain = min(SPEED * elapsed, 1).
const SPEED: f32 = 0.25;

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

/// Which pipeline the frame uses.
#[derive(Clone, Copy, PartialEq)]
pub enum Mode {
    Fetch,
    Pulling,
}

/// Chapter 10: the frame fed by fixed vertex fetch or storage reads.
pub struct VertexPulling {
    pipelines: [RenderPipeline; 2],
    uniform_buffers: [Buffer; 2],
    params_bind_groups: [BindGroup; 2],
    storage_bind_group: BindGroup,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    mode: Mode,
    elapsed: f32,
    paused: bool,
    last_instant: Option<Instant>,
}

impl Sample for VertexPulling {
    fn init(gpu: &Gpu) -> Result<Self, Box<dyn Error>> {
        // Storage buffers per stage are a device limit; check up front.
        if gpu.device.limits().max_storage_buffers_per_shader_stage == 0 {
            return Err(
                "This example requires at least one storage buffer in the vertex stage".into(),
            );
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
                        min_binding_size: BufferSize::new(32),
                    },
                    count: None,
                }],
            });
        let storage_layout = gpu
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Vertex storage bind group layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let fetch_layout = gpu
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Fetch pipeline layout"),
                bind_group_layouts: &[Some(&params_layout)],
                immediate_size: 0,
            });
        let pulling_layout = gpu
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Pulling pipeline layout"),
                bind_group_layouts: &[Some(&params_layout), Some(&storage_layout)],
                immediate_size: 0,
            });
        let fetch_pipeline = gpu
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Vertex fetch pipeline"),
                layout: Some(&fetch_layout),
                vertex: VertexState {
                    module: &shader,
                    entry_point: Some("vs_fetch"),
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
        let pulling_pipeline = gpu
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Vertex pulling pipeline"),
                layout: Some(&pulling_layout),
                vertex: VertexState {
                    module: &shader,
                    entry_point: Some("vs_pull"),
                    buffers: &[],
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

        // One buffer serves both paths: fetch needs VERTEX, storage STORAGE.
        let vertex_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Shared vertex storage"),
            size: size_of_val(&VERTICES) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&VERTICES));
        let storage_bind_group = gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Vertex storage bind group"),
            layout: &storage_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(BufferBinding {
                    buffer: &vertex_buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        let uniform_buffers = [0, 1].map(|index| {
            gpu.device.create_buffer(&BufferDescriptor {
                label: Some(if index == 0 {
                    "Triangle 0 params"
                } else {
                    "Triangle 1 params"
                }),
                size: 32,
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        });
        let params_bind_groups = uniform_buffers.each_ref().map(|buffer| {
            gpu.device.create_bind_group(&BindGroupDescriptor {
                label: Some("Params bind group"),
                layout: &params_layout,
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

        let index_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Rectangle indices"),
            size: size_of_val(&INDICES) as u64,
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&index_buffer, 0, bytemuck::cast_slice(&INDICES));
        Ok(Self {
            pipelines: [fetch_pipeline, pulling_pipeline],
            uniform_buffers,
            params_bind_groups,
            storage_bind_group,
            vertex_buffer,
            index_buffer,
            mode: Mode::Fetch,
            elapsed: 0.0,
            paused: false,
            last_instant: None,
        })
    }

    fn draw(&mut self, gpu: &Gpu, encoder: &mut CommandEncoder, view: &TextureView) {
        let now = Instant::now();
        if let Some(last) = self.last_instant
            && !self.paused
        {
            self.elapsed += now.duration_since(last).as_secs_f32();
        }
        self.last_instant = Some(now);
        let gain = (SPEED * self.elapsed).min(1.0);
        let params = Params::with_gain(gain);
        let params_bytes = serialize(&params);
        for buffer in &self.uniform_buffers {
            gpu.queue.write_buffer(buffer, 0, &params_bytes);
        }

        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Vertex pulling pass"),
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
        let pipeline = match self.mode {
            Mode::Fetch => &self.pipelines[0],
            Mode::Pulling => &self.pipelines[1],
        };
        pass.set_pipeline(pipeline);
        if self.mode == Mode::Pulling {
            pass.set_bind_group(1, &self.storage_bind_group, &[]);
        } else {
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        }
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        pass.set_bind_group(0, &self.params_bind_groups[0], &[]);
        pass.draw_indexed(0..3, 0, 0..1);
        pass.set_bind_group(0, &self.params_bind_groups[1], &[]);
        pass.draw_indexed(3..6, 0, 0..1);
    }

    fn window_event(&mut self, window: &winit::window::Window, event: &WindowEvent) {
        if let WindowEvent::KeyboardInput {
            event: key_event, ..
        } = event
            && key_event.state == ElementState::Pressed
            && let PhysicalKey::Code(key_code) = key_event.physical_key
        {
            match key_code {
                KeyCode::Space => self.paused = !self.paused,
                KeyCode::KeyR => self.elapsed = 0.0,
                KeyCode::KeyF => {
                    self.mode = match self.mode {
                        Mode::Fetch => Mode::Pulling,
                        Mode::Pulling => Mode::Fetch,
                    };
                }
                _ => {}
            }
        }

        if matches!(event, WindowEvent::RedrawRequested) {
            window.request_redraw();
        }
    }
}

impl VertexPulling {
    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
    }

    pub fn set_elapsed(&mut self, seconds: f32) {
        self.elapsed = seconds;
        self.last_instant = None;
    }
}

fn serialize(params: &Params) -> Vec<u8> {
    let mut buffer = UniformBuffer::new(Vec::<u8>::new());
    buffer
        .write(params)
        .expect("params fit the uniform contract");
    buffer.into_inner()
}
