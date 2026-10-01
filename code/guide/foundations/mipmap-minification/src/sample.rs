use encase::UniformBuffer;
use shell::{Gpu, Sample};
use std::error::Error;
use std::time::Instant;
use wgpu::{
    AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferAddress, BufferBinding, BufferBindingType,
    BufferDescriptor, BufferSize, BufferUsages, Color, ColorTargetState, ColorWrites,
    CommandEncoder, FilterMode, FragmentState, FrontFace, LoadOp, MipmapFilterMode, Operations,
    PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor,
    SamplerBindingType, SamplerDescriptor, ShaderStages, StoreOp, TextureSampleType, TextureView,
    TextureViewDimension, VertexAttribute, VertexBufferLayout, VertexFormat, VertexState,
    VertexStepMode, include_wgsl,
};
use winit::event::{ElementState, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

use crate::params::Params;

/// 144 repeats: two texels per pixel on the 576-pixel quad, LOD ~1.
const K: f32 = 144.0;
/// phase = min(t, 2) / 8: a quarter-repeat slide over the first two seconds.
const PHASE_SPEED: f32 = 1.0 / 8.0;
const PHASE_DURATION: f32 = 2.0;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 4],
    color: [f32; 4],
}

const VERTICES: [Vertex; 4] = [
    Vertex {
        position: [-0.75, 0.75, 0.5, 1.0],
        color: [1.0, 1.0, 1.0, 1.0],
    },
    Vertex {
        position: [0.75, 0.75, 0.5, 1.0],
        color: [1.0, 1.0, 1.0, 1.0],
    },
    Vertex {
        position: [-0.75, -0.75, 0.5, 1.0],
        color: [1.0, 1.0, 1.0, 1.0],
    },
    Vertex {
        position: [0.75, -0.75, 0.5, 1.0],
        color: [1.0, 1.0, 1.0, 1.0],
    },
];

const UVS: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];

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

const UV_LAYOUT: VertexBufferLayout<'static> = VertexBufferLayout {
    array_stride: 8,
    step_mode: VertexStepMode::Vertex,
    attributes: &[VertexAttribute {
        format: VertexFormat::Float32x2,
        offset: 0,
        shader_location: 2,
    }],
};

/// Which mip level the sampler is forced to.
#[derive(Clone, Copy, PartialEq)]
pub enum LodClamp {
    Level0,
    Level1,
}

/// Chapter 14: checkerboard minification with and without prefiltering.
pub struct MipmapMinification {
    pipeline: RenderPipeline,
    params_buffer: Buffer,
    bind_groups: [BindGroup; 2],
    vertex_buffer: Buffer,
    uv_buffer: Buffer,
    index_buffer: Buffer,
    lod_clamp: LodClamp,
    elapsed: f32,
    paused: bool,
    last_instant: Option<Instant>,
}

impl Sample for MipmapMinification {
    fn init(gpu: &Gpu) -> Result<Self, Box<dyn Error>> {
        let shader = gpu
            .device
            .create_shader_module(include_wgsl!("shader.wgsl"));
        let layout = gpu
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Minification layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(16),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        let pipeline_layout = gpu
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Minification pipeline layout"),
                bind_group_layouts: &[Some(&layout)],
                immediate_size: 0,
            });
        let pipeline = gpu
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Minification pipeline"),
                layout: Some(&pipeline_layout),
                vertex: VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[Some(Vertex::LAYOUT), Some(UV_LAYOUT)],
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
        let (_texture, view) = crate::texture::create(gpu);
        let params_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Minification params"),
            size: 16,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let describe_sampler = |lod: f32| {
            gpu.device.create_sampler(&SamplerDescriptor {
                label: Some(if lod == 0.0 {
                    "LOD 0 sampler"
                } else {
                    "LOD 1 sampler"
                }),
                address_mode_u: AddressMode::Repeat,
                address_mode_v: AddressMode::Repeat,
                address_mode_w: AddressMode::Repeat,
                mag_filter: FilterMode::Nearest,
                min_filter: FilterMode::Nearest,
                mipmap_filter: MipmapFilterMode::Nearest,
                lod_min_clamp: lod,
                lod_max_clamp: lod,
                ..SamplerDescriptor::default()
            })
        };
        let samplers = [describe_sampler(0.0), describe_sampler(1.0)];
        let bind_groups = samplers.each_ref().map(|sampler| {
            gpu.device.create_bind_group(&BindGroupDescriptor {
                label: Some("Minification bind group"),
                layout: &layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(BufferBinding {
                            buffer: &params_buffer,
                            offset: 0,
                            size: None,
                        }),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(sampler),
                    },
                ],
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
        let uv_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Quad UVs"),
            size: size_of_val(&UVS) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&uv_buffer, 0, bytemuck::cast_slice(&UVS));
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
            params_buffer,
            bind_groups,
            vertex_buffer,
            uv_buffer,
            index_buffer,
            lod_clamp: LodClamp::Level1,
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
        let phase = self.elapsed.min(PHASE_DURATION) * PHASE_SPEED;
        let mut bytes = UniformBuffer::new(Vec::<u8>::new());
        bytes
            .write(&Params::new(K, phase))
            .expect("fits the uniform contract");
        let params_bytes = bytes.into_inner();
        gpu.queue
            .write_buffer(&self.params_buffer, 0, &params_bytes);

        let bind_group = match self.lod_clamp {
            LodClamp::Level0 => &self.bind_groups[0],
            LodClamp::Level1 => &self.bind_groups[1],
        };
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Minification pass"),
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
        pass.set_bind_group(0, bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_vertex_buffer(1, self.uv_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        pass.draw_indexed(0..6, 0, 0..1);
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
                KeyCode::KeyL => {
                    self.lod_clamp = match self.lod_clamp {
                        LodClamp::Level0 => LodClamp::Level1,
                        LodClamp::Level1 => LodClamp::Level0,
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

impl MipmapMinification {
    pub fn set_lod_clamp(&mut self, lod_clamp: LodClamp) {
        self.lod_clamp = lod_clamp;
    }

    pub fn set_elapsed(&mut self, seconds: f32) {
        self.elapsed = seconds;
        self.last_instant = None;
    }
}
