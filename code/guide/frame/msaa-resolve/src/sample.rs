use std::error::Error;

use encase::UniformBuffer;
use glam::{Mat4, Vec3};
use framework::{Gpu, Sample};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBinding, BufferBindingType, BufferDescriptor,
    BufferSize, BufferUsages, Color, ColorTargetState, ColorWrites, CommandEncoder,
    CompareFunction, DepthStencilState, Extent3d, FragmentState, FrontFace, LoadOp, Operations,
    PipelineCompilationOptions, PipelineLayout, PipelineLayoutDescriptor, PrimitiveState,
    PrimitiveTopology, RenderPassColorAttachment, RenderPassDepthStencilAttachment,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, ShaderModule, ShaderStages,
    StoreOp, Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType,
    TextureUsages, TextureView, TextureViewDescriptor, TextureViewDimension, VertexAttribute,
    VertexBufferLayout, VertexFormat, VertexState, VertexStepMode, include_wgsl,
};
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::Window;

use crate::params::Params;

/// Camera pose from chapter 20: eye five meters in front of the origin.
const EYE: Vec3 = Vec3::new(0.0, 0.0, 5.0);
/// The projection of chapter 19a: 60 degrees vertical, 4:3 fallback frame.
const FOV_Y: f32 = std::f32::consts::FRAC_PI_3;
const NEAR: f32 = 1.0;
const FAR: f32 = 9.0;
/// Fallback target size until the first resize; matches the default window.
const FALLBACK_SIZE: (u32, u32) = (800, 600);

const RED: [f32; 4] = [0.85, 0.25, 0.25, 1.0];
const BLUE: [f32; 4] = [0.25, 0.4, 0.85, 1.0];

/// The two sample counts; 4x is guaranteed by the WebGPU baseline.
const SAMPLE_COUNTS: [u32; 2] = [1, 4];

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 4],
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

/// Two triangles crossing along x = 0 (A in the z = 0 plane, B tilted
/// around Y); the depth attachment decides the crossing line.
const VERTICES: [Vertex; 6] = [
    Vertex {
        position: [-1.7, 0.1, 0.0, 1.0],
        color: RED,
    },
    Vertex {
        position: [1.7, 0.1, 0.0, 1.0],
        color: RED,
    },
    Vertex {
        position: [0.0, 2.6, 0.0, 1.0],
        color: RED,
    },
    Vertex {
        position: [-0.6, 0.4, -1.7, 1.0],
        color: BLUE,
    },
    Vertex {
        position: [0.6, 0.4, 1.7, 1.0],
        color: BLUE,
    },
    Vertex {
        position: [0.0, 2.8, 0.0, 1.0],
        color: BLUE,
    },
];

const INDICES: [u16; 6] = [0, 1, 2, 3, 4, 5];

/// Linear clear: the intermediate frame is a linear RGBA8Unorm target.
const CLEAR: Color = Color {
    r: 0.1,
    g: 0.1,
    b: 0.14,
    a: 1.0,
};

/// Intersecting triangles rendered into an intermediate frame, then shown
/// through a fullscreen pass; M walks the scene pass between 1 and 4 samples.
pub struct MsaaResolve {
    /// [0] draws direct; [1] draws the multisample pair and resolves.
    scene_pipelines: [RenderPipeline; 2],
    fullscreen_pipeline: RenderPipeline,
    /// Shape of the fullscreen input: the single-sample frame texture.
    frame_layout: BindGroupLayout,
    bind_group: BindGroup,
    params_buffer: Buffer,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    msaa_enabled: bool,
    /// Single-sample linear frame; always the fullscreen input.
    frame_texture: Option<Texture>,
    frame_view: Option<TextureView>,
    frame_bind_group: Option<BindGroup>,
    /// The count = 4 color attachment; resolve squeezes it into the frame.
    msaa_view: Option<TextureView>,
    /// Depth attachment per mode: count must match the pipeline.
    single_depth_view: Option<TextureView>,
    msaa_depth_view: Option<TextureView>,
    target_size: (u32, u32),
    /// Nonzero target size seen in the events; consumed by the next draw.
    pending_size: Option<PhysicalSize<u32>>,
    projection: Mat4,
}

impl Sample for MsaaResolve {
    fn init(gpu: &Gpu) -> Result<Self, Box<dyn Error>> {
        let scene_shader = gpu.device.create_shader_module(include_wgsl!("scene.wgsl"));
        let fullscreen_shader = gpu
            .device
            .create_shader_module(include_wgsl!("fullscreen.wgsl"));
        let layout = gpu
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("MSAA layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(128),
                    },
                    count: None,
                }],
            });
        let scene_pipeline_layout = gpu
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("MSAA scene pipeline layout"),
                bind_group_layouts: &[Some(&layout)],
                immediate_size: 0,
            });
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
        let fullscreen_pipeline_layout =
            gpu.device
                .create_pipeline_layout(&PipelineLayoutDescriptor {
                    label: Some("Fullscreen pipeline layout"),
                    bind_group_layouts: &[Some(&frame_layout)],
                    immediate_size: 0,
                });
        // The count is pipeline state: switching with M is a pipeline switch.
        let scene_pipelines = SAMPLE_COUNTS
            .map(|count| create_scene_pipeline(gpu, &scene_pipeline_layout, &scene_shader, count));
        // The final pass is a plain single-sample copy onto the surface.
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
                multisample: wgpu::MultisampleState::default(),
                cache: None,
                multiview_mask: None,
            });

        let params_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("MSAA params"),
            size: 128,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("MSAA bind group"),
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
            label: Some("Triangle vertices"),
            size: size_of_val(&VERTICES) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&VERTICES));
        let index_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Triangle indices"),
            size: size_of_val(&INDICES) as u64,
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&index_buffer, 0, bytemuck::cast_slice(&INDICES));
        Ok(Self {
            scene_pipelines,
            fullscreen_pipeline,
            frame_layout,
            bind_group,
            params_buffer,
            vertex_buffer,
            index_buffer,
            msaa_enabled: false,
            frame_texture: None,
            frame_view: None,
            frame_bind_group: None,
            msaa_view: None,
            single_depth_view: None,
            msaa_depth_view: None,
            target_size: (0, 0),
            pending_size: None,
            projection: glam::camera::rh::proj::directx::perspective(
                FOV_Y,
                FALLBACK_SIZE.0 as f32 / FALLBACK_SIZE.1 as f32,
                NEAR,
                FAR,
            ),
        })
    }

    /// Framework contract: called right after `init` and on every resize.
    fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.pending_size = Some(PhysicalSize::new(width, height));
            self.projection = glam::camera::rh::proj::directx::perspective(
                FOV_Y,
                width as f32 / height as f32,
                NEAR,
                FAR,
            );
        }
    }

    fn draw(&mut self, gpu: &Gpu, encoder: &mut CommandEncoder, view: &TextureView) {
        // A pending size (re)creates the attachment kit here, in draw.
        if let Some(size) = self.pending_size.take()
            && self.target_size != (size.width, size.height)
        {
            self.recreate_kit(gpu, size.width, size.height);
        }

        let params = Params {
            view: glam::camera::rh::view::look_at_mat4(EYE, Vec3::ZERO, Vec3::Y),
            proj: self.projection,
        };
        let mut bytes = UniformBuffer::new(Vec::<u8>::new());
        bytes.write(&params).expect("fits the uniform contract");
        gpu.queue
            .write_buffer(&self.params_buffer, 0, &bytes.into_inner());

        // Pass 1: the scene; MSAA mode resolves into the single-sample frame.
        {
            let mode = usize::from(self.msaa_enabled);
            let (color_view, depth_view, resolve_target) = if self.msaa_enabled {
                (
                    self.msaa_view
                        .as_ref()
                        .expect("msaa kit exists after recreate"),
                    self.msaa_depth_view
                        .as_ref()
                        .expect("msaa kit exists after recreate"),
                    Some(
                        self.frame_view
                            .as_ref()
                            .expect("frame view exists after recreate"),
                    ),
                )
            } else {
                (
                    self.frame_view
                        .as_ref()
                        .expect("frame view exists after recreate"),
                    self.single_depth_view
                        .as_ref()
                        .expect("single kit exists after recreate"),
                    None,
                )
            };
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("MSAA scene pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: color_view,
                    resolve_target,
                    ops: Operations {
                        load: LoadOp::Clear(CLEAR),
                        // Resolve happens at pass end regardless; the MSAA
                        // texture itself is never read again, so in the
                        // multisampled mode its store can be discarded.
                        store: if resolve_target.is_some() {
                            StoreOp::Discard
                        } else {
                            StoreOp::Store
                        },
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: depth_view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(1.0),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..RenderPassDescriptor::default()
            });
            pass.set_pipeline(&self.scene_pipelines[mode]);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            pass.draw_indexed(0..6, 0, 0..1);
        }

        // Pass 2: the fullscreen copy of the finished single-sample frame.
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Fullscreen to surface pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: Operations {
                    // Never visible: the triangle covers every pixel.
                    load: LoadOp::Clear(CLEAR),
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

    fn window_event(&mut self, window: &Window, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput {
                event: key_event, ..
            } if key_event.state == ElementState::Pressed
                && let PhysicalKey::Code(key_code) = key_event.physical_key
                // Static frame: repaint only when a key changed it.
                && key_code == KeyCode::KeyM =>
            {
                self.msaa_enabled = !self.msaa_enabled;
                window.request_redraw();
            }
            _ => {}
        }
    }
}

impl MsaaResolve {
    /// A new frame texture also means a new view and fullscreen bind group.
    fn recreate_kit(&mut self, gpu: &Gpu, width: u32, height: u32) {
        let size = Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        // 4x Rgba8Unorm is guaranteed baseline; 8x would need a query.
        let frame = gpu.device.create_texture(&TextureDescriptor {
            label: Some("Single-sample frame"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            // Linear between passes; sRGB encoding happens only at the surface.
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let frame_view = frame.create_view(&TextureViewDescriptor {
            label: Some("Single-sample frame view"),
            ..TextureViewDescriptor::default()
        });
        let frame_bind_group = gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Fullscreen bind group"),
            layout: &self.frame_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&frame_view),
            }],
        });
        let msaa_color = gpu.device.create_texture(&TextureDescriptor {
            label: Some("MSAA color"),
            size,
            mip_level_count: 1,
            sample_count: SAMPLE_COUNTS[1],
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let msaa_view = msaa_color.create_view(&TextureViewDescriptor {
            label: Some("MSAA color view"),
            ..TextureViewDescriptor::default()
        });
        let single_depth = gpu.device.create_texture(&TextureDescriptor {
            label: Some("Scene depth"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let single_depth_view = single_depth.create_view(&TextureViewDescriptor {
            label: Some("Scene depth view"),
            ..TextureViewDescriptor::default()
        });
        // A count = 4 pass needs count = 4 depth, or validation fails.
        let msaa_depth = gpu.device.create_texture(&TextureDescriptor {
            label: Some("MSAA depth"),
            size,
            mip_level_count: 1,
            sample_count: SAMPLE_COUNTS[1],
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let msaa_depth_view = msaa_depth.create_view(&TextureViewDescriptor {
            label: Some("MSAA depth view"),
            ..TextureViewDescriptor::default()
        });
        self.frame_texture = Some(frame);
        self.frame_view = Some(frame_view);
        self.frame_bind_group = Some(frame_bind_group);
        self.msaa_view = Some(msaa_view);
        self.single_depth_view = Some(single_depth_view);
        self.msaa_depth_view = Some(msaa_depth_view);
        self.target_size = (width, height);
    }

    /// The verification crate drives this directly.
    pub fn set_msaa(&mut self, enabled: bool) {
        self.msaa_enabled = enabled;
    }

    /// The current mode, for diagnostics.
    pub fn msaa_enabled(&self) -> bool {
        self.msaa_enabled
    }
}

fn create_scene_pipeline(
    gpu: &Gpu,
    layout: &PipelineLayout,
    shader: &ShaderModule,
    sample_count: u32,
) -> RenderPipeline {
    gpu.device
        .create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("MSAA scene pipeline"),
            layout: Some(layout),
            vertex: VertexState {
                module: shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(Vertex::LAYOUT)],
                compilation_options: PipelineCompilationOptions::default(),
            },
            fragment: Some(FragmentState {
                module: shader,
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
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            // The pipeline declares the pass's sample count; all else is identical.
            multisample: wgpu::MultisampleState {
                count: sample_count,
                ..Default::default()
            },
            cache: None,
            multiview_mask: None,
        })
}
