use std::error::Error;

use encase::UniformBuffer;
use framework::{Gpu, Sample};
use glam::{Mat4, Vec3};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBinding, BufferBindingType, BufferDescriptor,
    BufferSize, BufferUsages, Color, ColorTargetState, ColorWrites, CommandEncoder, Extent3d,
    FragmentState, FrontFace, LoadOp, Operations, PipelineCompilationOptions,
    PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology, RenderPassColorAttachment,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, ShaderStages, StoreOp, Texture,
    TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType, TextureUsages,
    TextureView, TextureViewDescriptor, TextureViewDimension, VertexState, include_wgsl,
};
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::Window;

use crate::mesh::{INDICES, Vertex, plane_vertices};
use crate::params::{EXPOSURES, INTENSITIES, Light, Material, Params, ToneParams};
use bytemuck::cast_slice;
use glam::camera::rh::proj::directx::perspective;
use glam::camera::rh::view::look_at_mat4;
use std::f32::consts::FRAC_PI_3;
use wgpu::BindingResource;
use wgpu::IndexFormat;
use wgpu::MultisampleState;

const FOV_Y: f32 = FRAC_PI_3;
const NEAR: f32 = 0.1;
const FAR: f32 = 50.0;

/// Fallback target size until the first resize; matches the default window.
const FALLBACK_SIZE: (u32, u32) = (800, 600);

/// The camera of chapter 25: straight in front of the plane.
const EYE: Vec3 = Vec3::new(0.0, 0.0, 3.0);

/// Chapter start indices: exposure 1.0, the chapter light intensity.
const EXPOSURE_START: usize = 1;
const INTENSITY_START: usize = 0;

/// The Blinn-Phong plane rendered into RGBA16Float, then shown through
/// a fullscreen tone mapper: exposure, Reinhard, one sRGB encoding.
pub struct HdrOutput {
    scene_pipeline: RenderPipeline,
    tone_pipeline: RenderPipeline,
    /// Shape of the tone pass group: uniform first, HDR frame second.
    tone_layout: BindGroupLayout,
    tone_bind_group: Option<BindGroup>,
    scene_bind_group: BindGroup,
    params_buffer: Buffer,
    tone_buffer: Buffer,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    hdr_texture: Option<Texture>,
    hdr_view: Option<TextureView>,
    target_size: (u32, u32),
    /// Nonzero target size seen in the events; consumed by the next draw.
    pending_size: Option<PhysicalSize<u32>>,
    exposure_index: usize,
    intensity_index: usize,
    clipping: bool,
    projection: Mat4,
}

impl Sample for HdrOutput {
    fn init(gpu: &Gpu) -> Result<Self, Box<dyn Error>> {
        let vertices = plane_vertices();
        let scene_shader = gpu.device.create_shader_module(include_wgsl!("scene.wgsl"));
        let tone_shader = gpu.device.create_shader_module(include_wgsl!("tone.wgsl"));
        let scene_layout = gpu
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("HDR scene layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(144),
                    },
                    count: None,
                }],
            });
        // The tone pass reads the uniform and the HDR frame in one group.
        let tone_layout = gpu
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Tone layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(8),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: false },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            });
        let scene_pipeline_layout = gpu
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("HDR scene pipeline layout"),
                bind_group_layouts: &[Some(&scene_layout)],
                immediate_size: 0,
            });
        let tone_pipeline_layout = gpu
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Tone pipeline layout"),
                bind_group_layouts: &[Some(&tone_layout)],
                immediate_size: 0,
            });
        let scene_pipeline = gpu
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("HDR scene pipeline"),
                layout: Some(&scene_pipeline_layout),
                vertex: VertexState {
                    module: &scene_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[Some(Vertex::LAYOUT)],
                    compilation_options: PipelineCompilationOptions::default(),
                },
                fragment: Some(FragmentState {
                    module: &scene_shader,
                    entry_point: Some("fs_main"),
                    // The float target stores values above 1.0 unclamped.
                    targets: &[Some(ColorTargetState {
                        format: TextureFormat::Rgba16Float,
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
        let tone_pipeline = gpu
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Tone map pipeline"),
                layout: Some(&tone_pipeline_layout),
                vertex: VertexState {
                    module: &tone_shader,
                    entry_point: Some("vs_full"),
                    buffers: &[],
                    compilation_options: PipelineCompilationOptions::default(),
                },
                fragment: Some(FragmentState {
                    module: &tone_shader,
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

        let params_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("HDR scene params"),
            size: 144,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let scene_bind_group = gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("HDR scene bind group"),
            layout: &scene_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: &params_buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });
        let tone_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Tone params"),
            // Eight bytes: a multiple of four, exactly the uniform size.
            size: 8,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let vertex_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Plane vertices"),
            size: size_of_val(&vertices) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&vertex_buffer, 0, cast_slice(&vertices));
        let index_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Plane indices"),
            size: size_of_val(&INDICES) as u64,
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&index_buffer, 0, cast_slice(&INDICES));
        Ok(Self {
            scene_pipeline,
            tone_pipeline,
            tone_layout,
            tone_bind_group: None,
            scene_bind_group,
            params_buffer,
            tone_buffer,
            vertex_buffer,
            index_buffer,
            hdr_texture: None,
            hdr_view: None,
            target_size: (0, 0),
            pending_size: None,
            exposure_index: EXPOSURE_START,
            intensity_index: INTENSITY_START,
            clipping: false,
            projection: perspective(
                FOV_Y,
                FALLBACK_SIZE.0 as f32 / FALLBACK_SIZE.1 as f32,
                NEAR,
                FAR,
            ),
        })
    }

    ///  Framework contract: called right after `init` and on every resize.
    fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.pending_size = Some(PhysicalSize::new(width, height));
            self.projection = perspective(FOV_Y, width as f32 / height as f32, NEAR, FAR);
        }
    }

    fn draw(&mut self, gpu: &Gpu, encoder: &mut CommandEncoder, view: &TextureView) {
        // A pending size (re)creates the HDR frame here, in draw.
        if let Some(size) = self.pending_size.take()
            && self.target_size != (size.width, size.height)
        {
            self.recreate_hdr(gpu, size.width, size.height);
        }
        let hdr_view = self
            .hdr_view
            .as_ref()
            .expect("HDR view exists after recreate");

        // Scene uniforms; only the intensity can leave the SDR range.
        let params = Params::new(
            self.projection * look_at_mat4(EYE, Vec3::ZERO, Vec3::Y),
            EYE,
            Light {
                light_dir: Vec3::Z,
                intensity: INTENSITIES[self.intensity_index],
            },
            Material::CHAPTER,
        );
        let mut bytes = UniformBuffer::new(Vec::<u8>::new());
        bytes.write(&params).expect("fits the uniform contract");
        gpu.queue
            .write_buffer(&self.params_buffer, 0, &bytes.into_inner());

        // Pass 1: the scene into the linear HDR frame.
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Scene into HDR pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: hdr_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.12,
                            a: 1.0,
                        }),
                        store: StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                ..RenderPassDescriptor::default()
            });
            pass.set_pipeline(&self.scene_pipeline);
            pass.set_bind_group(0, &self.scene_bind_group, &[]);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint16);
            pass.draw_indexed(0..INDICES.len() as u32, 0, 0..1);
        }

        // Pass 2: exposure, tone mapping and the single sRGB encoding.
        let tone = ToneParams {
            exposure: EXPOSURES[self.exposure_index],
            mode: if self.clipping {
                ToneParams::CLIPPING
            } else {
                ToneParams::TONE
            },
        };
        let mut bytes = UniformBuffer::new(Vec::<u8>::new());
        bytes.write(&tone).expect("fits the uniform contract");
        gpu.queue
            .write_buffer(&self.tone_buffer, 0, &bytes.into_inner());
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Tone map to surface pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: Operations {
                    // Never visible: the triangle covers every pixel.
                    load: LoadOp::Clear(Color {
                        r: 0.1,
                        g: 0.1,
                        b: 0.12,
                        a: 1.0,
                    }),
                    store: StoreOp::Store,
                },
                depth_slice: None,
            })],
            ..RenderPassDescriptor::default()
        });
        pass.set_pipeline(&self.tone_pipeline);
        pass.set_bind_group(
            0,
            self.tone_bind_group.as_ref().expect("bind group exists"),
            &[],
        );
        pass.draw(0..3, 0..1);
    }

    fn window_event(&mut self, window: &Window, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput {
                event: key_event, ..
            } if key_event.state == ElementState::Pressed
                && let PhysicalKey::Code(key_code) = key_event.physical_key =>
            {
                // Static frame: repaint only when a key changed it.
                match key_code {
                    KeyCode::BracketLeft => {
                        self.exposure_index =
                            (self.exposure_index + EXPOSURES.len() - 1) % EXPOSURES.len();
                        window.request_redraw();
                    }
                    KeyCode::BracketRight => {
                        self.exposure_index = (self.exposure_index + 1) % EXPOSURES.len();
                        window.request_redraw();
                    }
                    KeyCode::KeyI => {
                        self.intensity_index = (self.intensity_index + 1) % INTENSITIES.len();
                        window.request_redraw();
                    }
                    KeyCode::KeyD => {
                        self.clipping = !self.clipping;
                        window.request_redraw();
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

impl HdrOutput {
    /// A new texture means a new view and tone bind group.
    fn recreate_hdr(&mut self, gpu: &Gpu, width: u32, height: u32) {
        let texture = gpu.device.create_texture(&TextureDescriptor {
            label: Some("HDR frame"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            // No sRGB variant of a float format; values stay linear.
            format: TextureFormat::Rgba16Float,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&TextureViewDescriptor {
            label: Some("HDR frame view"),
            ..TextureViewDescriptor::default()
        });
        let bind_group = gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Tone bind group"),
            layout: &self.tone_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &self.tone_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&view),
                },
            ],
        });
        self.hdr_texture = Some(texture);
        self.hdr_view = Some(view);
        self.tone_bind_group = Some(bind_group);
        self.target_size = (width, height);
    }

    /// The verification crate drives this directly.
    pub fn set_exposure(&mut self, index: usize) {
        self.exposure_index = index % EXPOSURES.len();
    }

    /// The verification crate drives this directly.
    pub fn set_intensity(&mut self, index: usize) {
        self.intensity_index = index % INTENSITIES.len();
    }

    /// The verification crate drives this directly.
    pub fn set_clipping(&mut self, clipping: bool) {
        self.clipping = clipping;
    }
}
