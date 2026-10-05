use std::error::Error;

use bytemuck::cast_slice;

use encase::UniformBuffer;
use framework::{Gpu, Sample};
use glam::Mat4;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBinding, BufferBindingType,
    BufferDescriptor, BufferSize, BufferUsages, Color, ColorTargetState, ColorWrites,
    CommandEncoder, CompareFunction, DepthStencilState, Extent3d, FragmentState, FrontFace,
    IndexFormat, LoadOp, MultisampleState, Operations, PipelineCompilationOptions,
    PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, ShaderStages, StoreOp, Texture, TextureDescriptor, TextureDimension,
    TextureFormat, TextureSampleType, TextureUsages, TextureView, TextureViewDescriptor,
    TextureViewDimension, VertexState, include_wgsl,
};
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::Window;

use crate::buffers::{MainBuffers, create_main_bind_group, create_uniform};
use crate::mesh::{CUBE_INDICES, FLOOR_INDICES, INDICES, VERTICES, Vertex};
use crate::params::{BIASES, SceneParams};
use crate::scene;

/// The R key map sizes; both exist for the whole run.
const SHADOW_SIZES: [u32; 2] = [1024, 512];
/// The M key cycle for the cube X offset: rest, +1, -1.
const CUBE_OFFSETS: [f32; 3] = [0.0, 1.0, -1.0];
/// The scene, cameras and comparison of 31a plus three switches:
/// receiver bias (B), map size (R), 3x3 PCF (P).
pub struct ShadowPcf {
    depth_pipeline: RenderPipeline,
    main_pipeline: RenderPipeline,
    /// One per (object, map size): models and shadow views differ.
    floor_bind_groups: [BindGroup; 2],
    cube_main_bind_groups: [BindGroup; 2],
    /// Depth-pass group: model + light matrix; no shadow map bound.
    cube_shadow_bind_group: BindGroup,
    camera_buffer: Buffer,
    light_buffer: Buffer,
    params_buffer: Buffer,
    floor_model_buffer: Buffer,
    cube_model_buffer: Buffer,
    params: SceneParams,
    /// Index into CUBE_OFFSETS; the M key cycles it.
    cube_offset: usize,
    /// Index into BIASES; the B key cycles it.
    bias_index: usize,
    /// Index into SHADOW_SIZES; the R key toggles it.
    map_index: usize,
    /// The P key state, mirrored into params.pcf.
    pcf: bool,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    shadow_views: [TextureView; 2],
    depth_texture: Option<Texture>,
    depth_view: Option<TextureView>,
    depth_size: (u32, u32),
    /// Nonzero target size seen in the events; consumed by the next draw.
    pending_size: Option<PhysicalSize<u32>>,
}

impl Sample for ShadowPcf {
    fn init(gpu: &Gpu) -> Result<Self, Box<dyn Error>> {
        let shader = gpu
            .device
            .create_shader_module(include_wgsl!("shader.wgsl"));
        let main_layout = gpu
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Shadow PCF main layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(64),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::VERTEX,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(64),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(64),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(64),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility: ShaderStages::FRAGMENT,
                        // A depth texture: textureLoad returns the single f32.
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Depth,
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            });
        // Uses only slots 0 and 2; numbers match the shader bindings.
        let depth_layout = gpu
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Shadow PCF depth layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(64),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::VERTEX,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(64),
                        },
                        count: None,
                    },
                ],
            });
        let main_pipeline_layout = gpu
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Shadow PCF main pipeline layout"),
                bind_group_layouts: &[Some(&main_layout)],
                immediate_size: 0,
            });
        let depth_pipeline_layout = gpu
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Shadow PCF depth pipeline layout"),
                bind_group_layouts: &[Some(&depth_layout)],
                immediate_size: 0,
            });
        let depth_pipeline = gpu
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Shadow depth pipeline"),
                layout: Some(&depth_pipeline_layout),
                vertex: VertexState {
                    module: &shader,
                    entry_point: Some("vs_depth"),
                    buffers: &[Some(Vertex::LAYOUT)],
                    compilation_options: PipelineCompilationOptions::default(),
                },
                // No fragment stage: depth only.
                fragment: None,
                primitive: PrimitiveState {
                    topology: PrimitiveTopology::TriangleList,
                    front_face: FrontFace::Ccw,
                    cull_mode: None,
                    ..PrimitiveState::default()
                },
                // The pipeline must declare the attachment's depth format.
                depth_stencil: Some(DepthStencilState {
                    format: TextureFormat::Depth32Float,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(CompareFunction::Less),
                    stencil: Default::default(),
                    // Raster-side bias stays zero: one unknown at a time.
                    bias: Default::default(),
                }),
                multisample: MultisampleState::default(),
                cache: None,
                multiview_mask: None,
            });
        let main_pipeline = gpu
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Shadow PCF main pipeline"),
                layout: Some(&main_pipeline_layout),
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
                // Depth test: the closer surface wins regardless of draw order.
                depth_stencil: Some(DepthStencilState {
                    format: TextureFormat::Depth32Float,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(CompareFunction::Less),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: MultisampleState::default(),
                cache: None,
                multiview_mask: None,
            });

        let camera_buffer = create_uniform(gpu, "Camera view*projection", 64);
        let light_buffer = create_uniform(gpu, "Light view*projection", 64);
        let params_buffer = create_uniform(gpu, "Shadow PCF scene params", 64);
        let shared_buffers = MainBuffers {
            layout: &main_layout,
            camera: &camera_buffer,
            light: &light_buffer,
            params: &params_buffer,
        };
        let floor_model_buffer = create_uniform(gpu, "Floor model (identity)", 64);
        let cube_model_buffer = create_uniform(gpu, "Cube model (translation)", 64);

        // Both maps exist up front; each draw clears and reads the selected one.
        let shadow_views = SHADOW_SIZES.map(|size| {
            let texture = gpu.device.create_texture(&TextureDescriptor {
                label: Some("Shadow map"),
                size: Extent3d {
                    width: size,
                    height: size,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Depth32Float,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            texture.create_view(&TextureViewDescriptor {
                label: Some("Shadow map view"),
                ..Default::default()
            })
        });
        let floor_bind_groups = shadow_views.each_ref().map(|shadow| {
            create_main_bind_group(
                gpu,
                "Floor main bind group",
                &floor_model_buffer,
                &shared_buffers,
                shadow,
            )
        });
        let cube_main_bind_groups = shadow_views.each_ref().map(|shadow| {
            create_main_bind_group(
                gpu,
                "Cube main bind group",
                &cube_model_buffer,
                &shared_buffers,
                shadow,
            )
        });
        let cube_shadow_bind_group = gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Cube caster bind group"),
            layout: &depth_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &cube_model_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &light_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });

        let vertex_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Floor and cube vertices"),
            size: size_of_val(&VERTICES) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&vertex_buffer, 0, cast_slice(&VERTICES));
        let index_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Floor and cube indices"),
            size: size_of_val(&INDICES) as u64,
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&index_buffer, 0, cast_slice(&INDICES));

        Ok(Self {
            depth_pipeline,
            main_pipeline,
            floor_bind_groups,
            cube_main_bind_groups,
            cube_shadow_bind_group,
            camera_buffer,
            light_buffer,
            params_buffer,
            floor_model_buffer,
            cube_model_buffer,
            params: SceneParams::chapter(),
            cube_offset: 0,
            bias_index: 0,
            map_index: 0,
            pcf: false,
            vertex_buffer,
            index_buffer,
            shadow_views,
            depth_texture: None,
            depth_view: None,
            depth_size: (0, 0),
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
        // A pending size (re)creates the depth attachment here, in draw.
        if let Some(size) = self.pending_size.take()
            && self.depth_size != (size.width, size.height)
        {
            self.recreate_depth(gpu, size.width, size.height);
        }

        // Fixed cameras; only the cube model follows the M key.
        let mut bytes = UniformBuffer::new(Vec::<u8>::new());
        bytes
            .write(&scene::camera_view_proj())
            .expect("fits the uniform contract");
        gpu.queue
            .write_buffer(&self.camera_buffer, 0, &bytes.into_inner());
        let mut bytes = UniformBuffer::new(Vec::<u8>::new());
        bytes
            .write(&scene::light_view_proj())
            .expect("fits the uniform contract");
        gpu.queue
            .write_buffer(&self.light_buffer, 0, &bytes.into_inner());
        let mut bytes = UniformBuffer::new(Vec::<u8>::new());
        bytes
            .write(&self.params)
            .expect("fits the uniform contract");
        gpu.queue
            .write_buffer(&self.params_buffer, 0, &bytes.into_inner());
        let mut bytes = UniformBuffer::new(Vec::<u8>::new());
        bytes
            .write(&Mat4::IDENTITY)
            .expect("fits the uniform contract");
        gpu.queue
            .write_buffer(&self.floor_model_buffer, 0, &bytes.into_inner());
        let mut bytes = UniformBuffer::new(Vec::<u8>::new());
        bytes
            .write(&scene::cube_model(CUBE_OFFSETS[self.cube_offset]))
            .expect("fits the uniform contract");
        gpu.queue
            .write_buffer(&self.cube_model_buffer, 0, &bytes.into_inner());

        // Pass 1: depth only, from the light, into the selected map.
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Shadow depth pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &self.shadow_views[self.map_index],
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(1.0),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..RenderPassDescriptor::default()
            });
            pass.set_pipeline(&self.depth_pipeline);
            pass.set_bind_group(0, &self.cube_shadow_bind_group, &[]);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint16);
            // Only the cube casts shadows.
            pass.draw_indexed(CUBE_INDICES, 0, 0..1);
        }

        // Pass 2: per-object model groups, all pointing at the selected map.
        let map = self.map_index;
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Shadow PCF main pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view,
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
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: self
                    .depth_view
                    .as_ref()
                    .expect("depth exists after recreate"),
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(1.0),
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..RenderPassDescriptor::default()
        });
        pass.set_pipeline(&self.main_pipeline);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint16);
        pass.set_bind_group(0, &self.floor_bind_groups[map], &[]);
        pass.draw_indexed(FLOOR_INDICES, 0, 0..1);
        pass.set_bind_group(0, &self.cube_main_bind_groups[map], &[]);
        pass.draw_indexed(CUBE_INDICES, 0, 0..1);
    }

    fn window_event(&mut self, window: &Window, event: &WindowEvent) {
        if let WindowEvent::KeyboardInput {
            event: key_event, ..
        } = event
            && key_event.state == ElementState::Pressed
            && let PhysicalKey::Code(key_code) = key_event.physical_key
        {
            // Static frame: redraw only when a key changed it.
            match key_code {
                KeyCode::KeyH => {
                    self.params.shadows_on = 1 - self.params.shadows_on;
                    window.request_redraw();
                }
                KeyCode::KeyM => {
                    self.cube_offset = (self.cube_offset + 1) % CUBE_OFFSETS.len();
                    window.request_redraw();
                }
                KeyCode::KeyB => {
                    self.bias_index = (self.bias_index + 1) % BIASES.len();
                    self.params.bias = BIASES[self.bias_index];
                    window.request_redraw();
                }
                KeyCode::KeyR => {
                    self.map_index = 1 - self.map_index;
                    window.request_redraw();
                }
                KeyCode::KeyP => {
                    self.pcf = !self.pcf;
                    self.params.pcf = u32::from(self.pcf);
                    window.request_redraw();
                }
                _ => {}
            }
        }
    }
}

impl ShadowPcf {
    /// Recreates the local depth attachment for a new target size.
    fn recreate_depth(&mut self, gpu: &Gpu, width: u32, height: u32) {
        let texture = gpu.device.create_texture(&TextureDescriptor {
            label: Some("Shadow PCF depth"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&TextureViewDescriptor {
            label: Some("Shadow PCF depth view"),
            ..Default::default()
        });
        self.depth_texture = Some(texture);
        self.depth_view = Some(view);
        self.depth_size = (width, height);
    }

    /// The verification crate drives this directly.
    pub fn set_shadows(&mut self, shadows: bool) {
        self.params.shadows_on = u32::from(shadows);
    }

    /// The verification crate drives this directly.
    pub fn set_cube_offset(&mut self, offset_x: f32) {
        if let Some(index) = CUBE_OFFSETS.iter().position(|&offset| offset == offset_x) {
            self.cube_offset = index;
        }
    }

    /// The verification crate drives this directly.
    pub fn set_bias(&mut self, bias: f32) {
        self.params.bias = bias;
        self.bias_index = BIASES.iter().position(|&value| value == bias).unwrap_or(0);
    }

    /// The verification crate drives this directly.
    pub fn set_map_size(&mut self, size: u32) {
        if let Some(index) = SHADOW_SIZES.iter().position(|&value| value == size) {
            self.map_index = index;
        }
    }

    /// The verification crate drives this directly.
    pub fn set_pcf(&mut self, pcf: bool) {
        self.pcf = pcf;
        self.params.pcf = u32::from(pcf);
    }
}
