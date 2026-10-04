use std::error::Error;

use framework::{Gpu, Sample};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferDescriptor, BufferUsages, Color,
    ColorTargetState, ColorWrites, CommandEncoder, ComputePassDescriptor, ComputePipeline,
    ComputePipelineDescriptor, Extent3d, FragmentState, FrontFace, LoadOp, Operations,
    PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor,
    StorageTextureAccess, StoreOp, Texture, TextureDescriptor, TextureDimension, TextureFormat,
    TextureSampleType, TextureUsages, TextureView, TextureViewDescriptor, TextureViewDimension,
    VertexAttribute, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode, include_wgsl,
};
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::Window;

/// Which fork's result the final pass shows; the P key switches.
#[derive(Clone, Copy, PartialEq)]
pub enum OutputMode {
    Fragment,
    Compute,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 4],
}

const VERTICES: [Vertex; 4] = [
    Vertex {
        position: [-0.75, 0.75, 0.5, 1.0],
    },
    Vertex {
        position: [0.75, 0.75, 0.5, 1.0],
    },
    Vertex {
        position: [-0.75, -0.75, 0.5, 1.0],
    },
    Vertex {
        position: [0.75, -0.75, 0.5, 1.0],
    },
];

/// v grows downward on screen, matching the top-to-bottom upload order.
const UVS: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];

const INDICES: [u16; 6] = [0, 1, 2, 2, 1, 3];

impl Vertex {
    const LAYOUT: VertexBufferLayout<'static> = VertexBufferLayout {
        array_stride: 16,
        step_mode: VertexStepMode::Vertex,
        attributes: &[VertexAttribute {
            format: VertexFormat::Float32x4,
            offset: 0,
            shader_location: 0,
        }],
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

/// A sized RGBA8Unorm image with its view and bind group.
struct Intermediate {
    _texture: Texture,
    view: TextureView,
    bind_group: BindGroup,
}

fn create_intermediate(
    gpu: &Gpu,
    layout: &BindGroupLayout,
    label: &str,
    width: u32,
    height: u32,
    usage: TextureUsages,
) -> Intermediate {
    let texture = gpu.device.create_texture(&TextureDescriptor {
        label: Some(label),
        size: Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba8Unorm,
        usage,
        view_formats: &[],
    });
    let view = texture.create_view(&TextureViewDescriptor {
        label: Some(label),
        ..TextureViewDescriptor::default()
    });
    let bind_group = gpu.device.create_bind_group(&BindGroupDescriptor {
        label: Some(label),
        layout,
        entries: &[BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(&view),
        }],
    });
    Intermediate {
        _texture: texture,
        view,
        bind_group,
    }
}

/// Two forks do the same halving of linear RGB: a fragment pipeline and a
/// compute pipeline writing a storage texture; P switches which is shown.
pub struct ImagePipeline {
    quad_pipeline: RenderPipeline,
    halve_pipeline: RenderPipeline,
    copy_pipeline: RenderPipeline,
    compute_pipeline: ComputePipeline,
    /// Shared shape of every plain float texture binding.
    frame_layout: BindGroupLayout,
    compute_layout: BindGroupLayout,
    source_bind_group: BindGroup,
    vertex_buffer: Buffer,
    uv_buffer: Buffer,
    index_buffer: Buffer,
    input: Option<Intermediate>,
    fragment_result: Option<Intermediate>,
    compute_result: Option<Intermediate>,
    compute_bind_group: Option<BindGroup>,
    size: (u32, u32),
    /// Nonzero target size seen in the events; consumed by the next draw.
    pending_size: Option<PhysicalSize<u32>>,
    mode: OutputMode,
}

impl Sample for ImagePipeline {
    fn init(gpu: &Gpu) -> Result<Self, Box<dyn Error>> {
        // Make the storage-texture contract explicit instead of failing in validation.
        let storage_usages = TextureFormat::Rgba8Unorm
            .guaranteed_format_features(wgpu::Features::empty())
            .allowed_usages;
        if !storage_usages.contains(TextureUsages::STORAGE_BINDING) {
            return Err("This example requires RGBA8Unorm storage textures".into());
        }

        let quad_shader = gpu.device.create_shader_module(include_wgsl!("quad.wgsl"));
        let halve_shader = gpu.device.create_shader_module(include_wgsl!("halve.wgsl"));
        let copy_shader = gpu.device.create_shader_module(include_wgsl!("copy.wgsl"));
        let halve_compute_shader = gpu
            .device
            .create_shader_module(include_wgsl!("halve_compute.wgsl"));
        let frame_layout = gpu
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Frame bind group layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                }],
            });
        let compute_layout = gpu
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Compute halve bind group layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: false },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::WriteOnly,
                            format: TextureFormat::Rgba8Unorm,
                            view_dimension: TextureViewDimension::D2,
                        },
                        count: None,
                    },
                ],
            });
        let quad_pipeline_layout = gpu
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Quad pipeline layout"),
                bind_group_layouts: &[Some(&frame_layout)],
                immediate_size: 0,
            });
        let halve_pipeline_layout = gpu
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Fragment halve pipeline layout"),
                bind_group_layouts: &[Some(&frame_layout)],
                immediate_size: 0,
            });
        let copy_pipeline_layout = gpu
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Copy pipeline layout"),
                bind_group_layouts: &[Some(&frame_layout)],
                immediate_size: 0,
            });
        let compute_pipeline_layout =
            gpu.device
                .create_pipeline_layout(&PipelineLayoutDescriptor {
                    label: Some("Compute halve pipeline layout"),
                    bind_group_layouts: &[Some(&compute_layout)],
                    immediate_size: 0,
                });

        let describe_render_pipeline = |label: &str,
                                        layout: &wgpu::PipelineLayout,
                                        shader: &wgpu::ShaderModule,
                                        vertex_entry: &str,
                                        fragment_entry: &str,
                                        target: TextureFormat,
                                        buffers: &[Option<VertexBufferLayout<'static>>]|
         -> RenderPipeline {
            gpu.device
                .create_render_pipeline(&RenderPipelineDescriptor {
                    label: Some(label),
                    layout: Some(layout),
                    vertex: VertexState {
                        module: shader,
                        entry_point: Some(vertex_entry),
                        buffers,
                        compilation_options: PipelineCompilationOptions::default(),
                    },
                    fragment: Some(FragmentState {
                        module: shader,
                        entry_point: Some(fragment_entry),
                        targets: &[Some(ColorTargetState {
                            format: target,
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
                })
        };
        let quad_pipeline = describe_render_pipeline(
            "Quad into input pipeline",
            &quad_pipeline_layout,
            &quad_shader,
            "vs_main",
            "fs_main",
            TextureFormat::Rgba8Unorm,
            &[Some(Vertex::LAYOUT), Some(UV_LAYOUT)],
        );
        let halve_pipeline = describe_render_pipeline(
            "Fragment halve pipeline",
            &halve_pipeline_layout,
            &halve_shader,
            "vs_full",
            "fs_halve",
            TextureFormat::Rgba8Unorm,
            &[],
        );
        let copy_pipeline = describe_render_pipeline(
            "Result to surface pipeline",
            &copy_pipeline_layout,
            &copy_shader,
            "vs_full",
            "fs_full",
            gpu.format,
            &[],
        );
        let compute_pipeline = gpu
            .device
            .create_compute_pipeline(&ComputePipelineDescriptor {
                label: Some("Compute halve pipeline"),
                layout: Some(&compute_pipeline_layout),
                module: &halve_compute_shader,
                entry_point: Some("halve"),
                compilation_options: PipelineCompilationOptions::default(),
                cache: None,
            });

        let (_texture, view) = crate::texture::create(gpu);
        let source_bind_group = gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Quad texture bind group"),
            layout: &frame_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            }],
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
            quad_pipeline,
            halve_pipeline,
            copy_pipeline,
            compute_pipeline,
            frame_layout,
            compute_layout,
            source_bind_group,
            vertex_buffer,
            uv_buffer,
            index_buffer,
            input: None,
            fragment_result: None,
            compute_result: None,
            compute_bind_group: None,
            size: (0, 0),
            pending_size: None,
            mode: OutputMode::Fragment,
        })
    }

    /// Framework contract: called right after `init` and on every resize.
    fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.pending_size = Some(PhysicalSize::new(width, height));
        }
    }

    fn draw(&mut self, gpu: &Gpu, encoder: &mut CommandEncoder, view: &TextureView) {
        if let Some(size) = self.pending_size.take()
            && self.size != (size.width, size.height)
        {
            self.recreate_intermediates(gpu, size.width, size.height);
        }
        let input = self.input.as_ref().expect("input exists after recreate");
        let fragment_result = self
            .fragment_result
            .as_ref()
            .expect("fragment result exists after recreate");
        let compute_result = self
            .compute_result
            .as_ref()
            .expect("compute result exists after recreate");
        let (width, height) = self.size;

        // Pass 1: the marked rectangle into the input image, as in 30a.
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Quad into input pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &input.view,
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
            pass.set_pipeline(&self.quad_pipeline);
            pass.set_bind_group(0, &self.source_bind_group, &[]);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.set_vertex_buffer(1, self.uv_buffer.slice(..));
            pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            pass.draw_indexed(0..6, 0, 0..1);
        }

        // Fork, fragment way: halve while rasterizing a fullscreen triangle.
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Fragment halve pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &fragment_result.view,
                    resolve_target: None,
                    ops: Operations {
                        // Never visible: the triangle covers every pixel.
                        load: LoadOp::Clear(Color::BLACK),
                        store: StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                ..RenderPassDescriptor::default()
            });
            pass.set_pipeline(&self.halve_pipeline);
            pass.set_bind_group(0, &input.bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        // Fork, compute way: invocation (x, y) halves texel (x, y).
        {
            let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("Compute halve pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.compute_pipeline);
            pass.set_bind_group(
                0,
                self.compute_bind_group
                    .as_ref()
                    .expect("compute bind group exists after recreate"),
                &[],
            );
            // Rounded up to whole 8x8 groups; the shader discards the surplus.
            pass.dispatch_workgroups(width.div_ceil(8), height.div_ceil(8), 1);
        }

        // Common output: show the selected result on the surface.
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Result to surface pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: Operations {
                    // Never visible: the triangle covers every pixel.
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
        pass.set_pipeline(&self.copy_pipeline);
        let shown = match self.mode {
            OutputMode::Fragment => &fragment_result.bind_group,
            OutputMode::Compute => &compute_result.bind_group,
        };
        pass.set_bind_group(0, shown, &[]);
        pass.draw(0..3, 0..1);
    }

    fn window_event(&mut self, window: &Window, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput {
                event: key_event, ..
            } if key_event.state == ElementState::Pressed
                && let PhysicalKey::Code(key_code) = key_event.physical_key
                && key_code == KeyCode::KeyP =>
            {
                self.mode = match self.mode {
                    OutputMode::Fragment => OutputMode::Compute,
                    OutputMode::Compute => OutputMode::Fragment,
                };
                window.request_redraw();
            }
            _ => {}
        }
    }
}

impl ImagePipeline {
    /// Rebuilds the intermediates and their bind groups for a new size.
    fn recreate_intermediates(&mut self, gpu: &Gpu, width: u32, height: u32) {
        let usage = TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING;
        self.input = Some(create_intermediate(
            gpu,
            &self.frame_layout,
            "Image pipeline input",
            width,
            height,
            usage,
        ));
        self.fragment_result = Some(create_intermediate(
            gpu,
            &self.frame_layout,
            "Fragment result",
            width,
            height,
            usage,
        ));
        // The compute output is never rendered into: storage replaces attachment.
        self.compute_result = Some(create_intermediate(
            gpu,
            &self.frame_layout,
            "Compute result",
            width,
            height,
            TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
        ));
        let compute_bind_group = gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Compute halve bind group"),
            layout: &self.compute_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &self.input.as_ref().expect("input just created").view,
                    ),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(
                        &self
                            .compute_result
                            .as_ref()
                            .expect("result just created")
                            .view,
                    ),
                },
            ],
        });
        self.compute_bind_group = Some(compute_bind_group);
        self.size = (width, height);
    }

    /// The verification crate drives this directly.
    pub fn set_mode(&mut self, mode: OutputMode) {
        self.mode = mode;
    }
}
