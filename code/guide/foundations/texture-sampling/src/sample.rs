use framework::{Gpu, Sample};
use std::error::Error;
use wgpu::{
    AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferAddress, BufferDescriptor, BufferUsages,
    Color, ColorTargetState, ColorWrites, CommandEncoder, FilterMode, FragmentState, FrontFace,
    LoadOp, Operations, PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState,
    PrimitiveTopology, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, StoreOp,
    TextureSampleType, TextureView, TextureViewDimension, VertexAttribute, VertexBufferLayout,
    VertexFormat, VertexState, VertexStepMode, include_wgsl,
};
use winit::event::{ElementState, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

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

/// Sampler state switchable at runtime: filter and address mode.
#[derive(Clone, Copy, PartialEq)]
pub enum Filter {
    Nearest,
    Linear,
}

impl Filter {
    /// Position in the samplers array, which lists [Nearest, Linear].
    fn index(self) -> usize {
        match self {
            Filter::Nearest => 0,
            Filter::Linear => 1,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Address {
    Clamp,
    Repeat,
}

/// Chapter 13b: UV sampling with predictable bilinear weights.
pub struct TextureSampling {
    pipeline: RenderPipeline,
    bind_groups: [[BindGroup; 2]; 2],
    vertex_buffer: Buffer,
    uv_buffer: Buffer,
    index_buffer: Buffer,
    filter: Filter,
    address: Address,
}

impl Sample for TextureSampling {
    fn init(gpu: &Gpu) -> Result<Self, Box<dyn Error>> {
        let shader = gpu
            .device
            .create_shader_module(include_wgsl!("shader.wgsl"));
        let texture_layout = gpu
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Texture sampling layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        let pipeline_layout = gpu
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Texture sampling pipeline layout"),
                bind_group_layouts: &[Some(&texture_layout)],
                immediate_size: 0,
            });
        let pipeline = gpu
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Texture sampling pipeline"),
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
        let samplers: [[Sampler; 2]; 2] = [Filter::Nearest, Filter::Linear].map(|filter| {
            [Address::Clamp, Address::Repeat].map(|address| {
                let label = format!(
                    "Quad sampler ({}, {})",
                    match filter {
                        Filter::Nearest => "nearest",
                        Filter::Linear => "linear",
                    },
                    match address {
                        Address::Clamp => "clamp",
                        Address::Repeat => "repeat",
                    }
                );
                gpu.device.create_sampler(&SamplerDescriptor {
                    label: Some(&label),
                    address_mode_u: address_mode(address),
                    address_mode_v: address_mode(address),
                    address_mode_w: address_mode(address),
                    mag_filter: filter_mode(filter),
                    min_filter: filter_mode(filter),
                    mipmap_filter: wgpu::MipmapFilterMode::Nearest,
                    ..SamplerDescriptor::default()
                })
            })
        });
        let bind_groups = samplers.each_ref().map(|row| {
            row.each_ref().map(|sampler| {
                gpu.device.create_bind_group(&BindGroupDescriptor {
                    label: Some("Texture sampling bind group"),
                    layout: &texture_layout,
                    entries: &[
                        BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&view),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(sampler),
                        },
                    ],
                })
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
            bind_groups,
            vertex_buffer,
            uv_buffer,
            index_buffer,
            filter: Filter::Linear,
            address: Address::Clamp,
        })
    }

    fn draw(&mut self, _gpu: &Gpu, encoder: &mut CommandEncoder, view: &TextureView) {
        let filter_index = self.filter.index();
        let address_index = usize::from(self.address == Address::Repeat);
        let bind_group = &self.bind_groups[filter_index][address_index];
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Texture sampling pass"),
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

    fn window_event(&mut self, _window: &winit::window::Window, event: &WindowEvent) {
        if let WindowEvent::KeyboardInput {
            event: key_event, ..
        } = event
            && key_event.state == ElementState::Pressed
            && let PhysicalKey::Code(key_code) = key_event.physical_key
        {
            match key_code {
                KeyCode::KeyF => {
                    self.filter = match self.filter {
                        Filter::Nearest => Filter::Linear,
                        Filter::Linear => Filter::Nearest,
                    };
                }
                KeyCode::KeyA => {
                    self.address = match self.address {
                        Address::Clamp => Address::Repeat,
                        Address::Repeat => Address::Clamp,
                    };
                }
                _ => {}
            }
        }
    }
}

fn filter_mode(filter: Filter) -> FilterMode {
    match filter {
        Filter::Nearest => FilterMode::Nearest,
        Filter::Linear => FilterMode::Linear,
    }
}

fn address_mode(address: Address) -> AddressMode {
    match address {
        Address::Clamp => AddressMode::ClampToEdge,
        Address::Repeat => AddressMode::Repeat,
    }
}

impl TextureSampling {
    pub fn set_filter(&mut self, filter: Filter) {
        self.filter = filter;
    }

    pub fn set_address(&mut self, address: Address) {
        self.address = address;
    }
}
