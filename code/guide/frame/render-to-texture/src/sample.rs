use std::error::Error;

use crate::quad::{INDICES, UV_LAYOUT, UVS, VERTICES, Vertex};
use framework::{Gpu, Sample};
use wgpu::{BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, Buffer,
    BufferDescriptor, BufferUsages, Color, ColorTargetState, ColorWrites, CommandEncoder,
    Extent3d, FragmentState, FrontFace, IndexFormat, LoadOp, MultisampleState, Operations,
    PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor,
    ShaderStages, StoreOp, Texture, TextureDescriptor, TextureDimension, TextureFormat,
    TextureSampleType, TextureUsages, TextureView, TextureViewDescriptor, TextureViewDimension,
    VertexState, include_wgsl,};
use winit::dpi::PhysicalSize;

use crate::texture::create;
use bytemuck::cast_slice;
/// then shows the finished frame on the surface.
pub struct RenderToTexture {
    quad_pipeline: RenderPipeline,
    fullscreen_pipeline: RenderPipeline,
    /// Shared shape of every plain float texture binding.
    frame_layout: BindGroupLayout,
    source_bind_group: BindGroup,
    vertex_buffer: Buffer,
    uv_buffer: Buffer,
    index_buffer: Buffer,
    frame_texture: Option<Texture>,
    frame_view: Option<TextureView>,
    frame_bind_group: Option<BindGroup>,
    frame_size: (u32, u32),
    /// Nonzero target size seen in the events; consumed by the next draw.
    pending_size: Option<PhysicalSize<u32>>,
}

impl Sample for RenderToTexture {
    fn init(gpu: &Gpu) -> Result<Self, Box<dyn Error>> {
        let quad_shader = gpu.device.create_shader_module(include_wgsl!("quad.wgsl"));
        let fullscreen_shader = gpu
            .device
            .create_shader_module(include_wgsl!("fullscreen.wgsl"));
        let frame_layout = gpu
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Frame bind group layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                }],
            });
        let quad_pipeline_layout = gpu
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Quad pipeline layout"),
                bind_group_layouts: &[Some(&frame_layout)],
                immediate_size: 0,
            });
        let fullscreen_pipeline_layout =
            gpu.device
                .create_pipeline_layout(&PipelineLayoutDescriptor {
                    label: Some("Fullscreen pipeline layout"),
                    bind_group_layouts: &[Some(&frame_layout)],
                    immediate_size: 0,
                });
        let quad_pipeline = gpu
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Quad into intermediate pipeline"),
                layout: Some(&quad_pipeline_layout),
                vertex: VertexState {
                    module: &quad_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[Some(Vertex::LAYOUT), Some(UV_LAYOUT)],
                    compilation_options: PipelineCompilationOptions::default(),
                },
                fragment: Some(FragmentState {
                    module: &quad_shader,
                    entry_point: Some("fs_main"),
                    // Linear target; sRGB encoding happens only in the fullscreen pass.
                    targets: &[Some(ColorTargetState {
                        format: TextureFormat::Rgba8Unorm,
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
        let fullscreen_pipeline = gpu
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Fullscreen copy pipeline"),
                layout: Some(&fullscreen_pipeline_layout),
                vertex: VertexState {
                    module: &fullscreen_shader,
                    entry_point: Some("vs_full"),
                    buffers: &[],
                    compilation_options: PipelineCompilationOptions::default(),
                },
                fragment: Some(FragmentState {
                    module: &fullscreen_shader,
                    entry_point: Some("fs_full"),
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

        let (_texture, view) = create(gpu);
        let source_bind_group = gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Quad texture bind group"),
            layout: &frame_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&view),
            }],
        });
        let vertex_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Quad vertices"),
            size: size_of_val(&VERTICES) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&vertex_buffer, 0, cast_slice(&VERTICES));
        let uv_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Quad UVs"),
            size: size_of_val(&UVS) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue.write_buffer(&uv_buffer, 0, cast_slice(&UVS));
        let index_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Quad indices"),
            size: size_of_val(&INDICES) as u64,
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&index_buffer, 0, cast_slice(&INDICES));
        Ok(Self {
            quad_pipeline,
            fullscreen_pipeline,
            frame_layout,
            source_bind_group,
            vertex_buffer,
            uv_buffer,
            index_buffer,
            frame_texture: None,
            frame_view: None,
            frame_bind_group: None,
            frame_size: (0, 0),
            pending_size: None,
        })
    }

    /// Framework contract: called right after `init` and on every resize.
    fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.pending_size = Some(PhysicalSize::new(width, height));
        }
    }

    fn draw(&mut self, gpu: &Gpu, encoder: &mut CommandEncoder, view: &TextureView) {
        // A pending size (re)creates the intermediate frame here, in draw.
        if let Some(size) = self.pending_size.take()
            && self.frame_size != (size.width, size.height)
        {
            self.recreate_frame(gpu, size.width, size.height);
        }
        let frame_view = self
            .frame_view
            .as_ref()
            .expect("frame view exists after recreate");

        // Pass 1: the rectangle into the intermediate texture; Store keeps it.
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Quad into intermediate pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: frame_view,
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
            pass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint16);
            pass.draw_indexed(0..6, 0, 0..1);
        }

        // Pass 2: copy the finished frame to the surface, one texel per pixel.
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Fullscreen to surface pass"),
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
        pass.set_pipeline(&self.fullscreen_pipeline);
        pass.set_bind_group(
            0,
            self.frame_bind_group.as_ref().expect("bind group exists"),
            &[],
        );
        pass.draw(0..3, 0..1);
    }
}

impl RenderToTexture {
    /// A new texture means a new view and bind group.
    fn recreate_frame(&mut self, gpu: &Gpu, width: u32, height: u32) {
        let texture = gpu.device.create_texture(&TextureDescriptor {
            label: Some("Intermediate frame"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            // Linear RGBA8Unorm: values stay linear between the passes.
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&TextureViewDescriptor {
            label: Some("Intermediate frame view"),
            ..TextureViewDescriptor::default()
        });
        let bind_group = gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Fullscreen bind group"),
            layout: &self.frame_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&view),
            }],
        });
        self.frame_texture = Some(texture);
        self.frame_view = Some(view);
        self.frame_bind_group = Some(bind_group);
        self.frame_size = (width, height);
    }
}
