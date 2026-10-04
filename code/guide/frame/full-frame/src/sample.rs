use std::error::Error;

use encase::{StorageBuffer, UniformBuffer};
use glam::{Mat4, Vec3};
use framework::{Gpu, Sample};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, BlendComponent, BlendFactor, BlendOperation, BlendState,
    Buffer, BufferBinding, BufferBindingType, BufferDescriptor, BufferSize, BufferUsages, Color,
    ColorTargetState, ColorWrites, CommandEncoder, CompareFunction, DepthStencilState, Extent3d,
    FragmentState, FrontFace, LoadOp, Operations, PipelineCompilationOptions, PipelineLayout,
    PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, ShaderModule, ShaderStages, StoreOp, Texture, TextureDescriptor,
    TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureView,
    TextureViewDescriptor, TextureViewDimension, VertexState, include_wgsl,
};
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::Window;

use crate::geometry::{self, CUBE_MESH, FLOOR_MESH, PANEL_INDICES, PanelVertex, Vertex};
use crate::scene::{self, ObjectRecord};

/// Fallback target size until the first resize; matches the default window.
const FALLBACK_SIZE: (u32, u32) = (800, 600);

/// The two sample counts; the M key walks between them.
const SAMPLE_COUNTS: [u32; 2] = [1, 4];

/// The HDR clear color, linear: tone mapping sees the background too.
const CLEAR: Color = Color {
    r: 0.1,
    g: 0.1,
    b: 0.12,
    a: 1.0,
};

/// Camera uniforms of the lit passes; encase rounds the struct to 80 bytes.
#[derive(encase::ShaderType, Debug, Clone, Copy)]
struct PassParams {
    view_proj: Mat4,
    shadows: u32,
}

/// Straight-alpha over: RGB weighed by the source alpha itself.
const STRAIGHT: BlendState = BlendState {
    color: BlendComponent {
        src_factor: BlendFactor::SrcAlpha,
        dst_factor: BlendFactor::OneMinusSrcAlpha,
        operation: BlendOperation::Add,
    },
    alpha: BlendComponent {
        src_factor: BlendFactor::One,
        dst_factor: BlendFactor::OneMinusSrcAlpha,
        operation: BlendOperation::Add,
    },
};

/// The finished frame as one explicit graph: shadow depth, opaque and
/// transparent HDR, resolve (with MSAA), tone map, surface.
pub struct FullFrame {
    shadow_pipeline: RenderPipeline,
    lit_pipelines: [RenderPipeline; 2],
    panel_pipelines: [RenderPipeline; 2],
    tone_pipeline: RenderPipeline,
    frame_layout: BindGroupLayout,
    camera_bind_group: BindGroup,
    light_bind_group: BindGroup,
    panel_bind_group: BindGroup,
    shadow_bind_group: BindGroup,
    objects_bind_group: BindGroup,
    material_bind_groups: [BindGroup; 2],
    camera_buffer: Buffer,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    panel_vertex_buffer: Buffer,
    panel_index_buffer: Buffer,
    shadow_view: TextureView,
    /// Single-sample HDR frame; always the tone mapper input.
    resolve_texture: Option<Texture>,
    resolve_view: Option<TextureView>,
    tone_bind_group: Option<BindGroup>,
    msaa_color_view: Option<TextureView>,
    single_depth_view: Option<TextureView>,
    msaa_depth_view: Option<TextureView>,
    target_size: (u32, u32),
    /// Nonzero target size seen in the events; consumed by the next draw.
    pending_size: Option<PhysicalSize<u32>>,
    msaa_enabled: bool,
    shadows_enabled: bool,
    projection: Mat4,
}

impl Sample for FullFrame {
    fn init(gpu: &Gpu) -> Result<Self, Box<dyn Error>> {
        if !gpu.device.features().contains(wgpu::Features::IMMEDIATES) {
            return Err("This example requires Features::IMMEDIATES (adapter support varies): the object index travels in the command state".into());
        }
        if gpu.device.limits().max_immediate_size < 4 {
            return Err(
                "This example requires max_immediate_size >= 4: one u32 object index per draw"
                    .into(),
            );
        }

        let scene_shader = gpu.device.create_shader_module(include_wgsl!("scene.wgsl"));
        let shadow_shader = gpu
            .device
            .create_shader_module(include_wgsl!("shadow.wgsl"));
        let panel_shader = gpu.device.create_shader_module(include_wgsl!("panel.wgsl"));
        let tone_shader = gpu.device.create_shader_module(include_wgsl!("tone.wgsl"));

        // Bindings by role: camera, material, objects, shadow, matrix, tone input.
        let pass_layout = gpu
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Pass layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(80),
                    },
                    count: None,
                }],
            });
        let material_layout = gpu
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Material layout"),
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
        let objects_layout = gpu
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Objects layout"),
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
        let shadow_layout = gpu
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Shadow layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(64),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Depth,
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            });
        let matrix_layout = gpu
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Matrix layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(64),
                    },
                    count: None,
                }],
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

        let lit_pipeline_layout = gpu
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Lit pipeline layout"),
                bind_group_layouts: &[
                    Some(&pass_layout),
                    Some(&material_layout),
                    Some(&objects_layout),
                    Some(&shadow_layout),
                ],
                immediate_size: 4,
            });
        // The None hole keeps group indices aligned with the lit shader.
        let shadow_pipeline_layout = gpu
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Shadow pipeline layout"),
                bind_group_layouts: &[Some(&matrix_layout), None, Some(&objects_layout)],
                immediate_size: 4,
            });
        let panel_pipeline_layout = gpu
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Panel pipeline layout"),
                bind_group_layouts: &[Some(&matrix_layout)],
                immediate_size: 0,
            });
        let tone_pipeline_layout = gpu
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Tone pipeline layout"),
                bind_group_layouts: &[Some(&frame_layout)],
                immediate_size: 0,
            });

        let shadow_pipeline = create_shadow_pipeline(gpu, &shadow_pipeline_layout, &shadow_shader);
        let lit_pipelines = SAMPLE_COUNTS
            .map(|count| create_lit_pipeline(gpu, &lit_pipeline_layout, &scene_shader, count));
        let panel_pipelines = SAMPLE_COUNTS
            .map(|count| create_panel_pipeline(gpu, &panel_pipeline_layout, &panel_shader, count));
        let tone_pipeline = create_tone_pipeline(gpu, &tone_pipeline_layout, &tone_shader);

        let camera_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Camera uniform"),
            size: 80,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let camera_bind_group = gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Camera bind group"),
            layout: &pass_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(BufferBinding {
                    buffer: &camera_buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });
        // Written once, bound twice: depth pass group 0 and the shadow group.
        let light_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Light uniform"),
            size: 64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        {
            let mut bytes = UniformBuffer::new(Vec::<u8>::new());
            bytes
                .write(&scene::light_view_proj())
                .expect("fits the uniform contract");
            gpu.queue
                .write_buffer(&light_buffer, 0, &bytes.into_inner());
        }
        let light_bind_group = gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Light bind group"),
            layout: &matrix_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(BufferBinding {
                    buffer: &light_buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });
        // The panel binds the camera buffer as a mat4: its first 64 bytes.
        let panel_bind_group = gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Panel bind group"),
            layout: &matrix_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(BufferBinding {
                    buffer: &camera_buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        // Static scene, uploaded once.
        let objects = scene::objects();
        let objects_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Object storage"),
            size: (scene::OBJECT_COUNT * size_of::<ObjectRecord>()) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        {
            let mut bytes = StorageBuffer::new(Vec::<u8>::new());
            bytes.write(&objects).expect("fits the storage contract");
            gpu.queue
                .write_buffer(&objects_buffer, 0, &bytes.into_inner());
        }
        let objects_bind_group = gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Objects bind group"),
            layout: &objects_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(BufferBinding {
                    buffer: &objects_buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        // Materials change by rebinding the group, not rewriting the data.
        let material_bind_groups = [scene::CUBE_MATERIAL, scene::FLOOR_MATERIAL].map(|material| {
            let buffer = gpu.device.create_buffer(&BufferDescriptor {
                label: Some("Material uniform"),
                size: 16,
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let mut bytes = UniformBuffer::new(Vec::<u8>::new());
            bytes.write(&material).expect("fits the uniform contract");
            gpu.queue.write_buffer(&buffer, 0, &bytes.into_inner());
            gpu.device.create_bind_group(&BindGroupDescriptor {
                label: Some("Material bind group"),
                layout: &material_layout,
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(BufferBinding {
                        buffer: &buffer,
                        offset: 0,
                        size: None,
                    }),
                }],
            })
        });

        // Single-sample regardless of the scene's sample count.
        let shadow_texture = gpu.device.create_texture(&TextureDescriptor {
            label: Some("Shadow map"),
            size: Extent3d {
                width: scene::SHADOW_MAP_SIZE,
                height: scene::SHADOW_MAP_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let shadow_view = shadow_texture.create_view(&TextureViewDescriptor {
            label: Some("Shadow map view"),
            ..TextureViewDescriptor::default()
        });
        let shadow_bind_group = gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Shadow bind group"),
            layout: &shadow_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(BufferBinding {
                        buffer: &light_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&shadow_view),
                },
            ],
        });

        // The shared mesh buffers: cube first, floor after it.
        let vertices = geometry::vertices();
        let indices = geometry::indices();
        let vertex_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Scene vertices"),
            size: size_of_val(&vertices) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        let index_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Scene indices"),
            size: size_of_val(&indices) as u64,
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&index_buffer, 0, bytemuck::cast_slice(&indices));

        let panel_vertices = geometry::panel_vertices();
        let panel_vertex_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Panel vertices"),
            size: size_of_val(&panel_vertices) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue.write_buffer(
            &panel_vertex_buffer,
            0,
            bytemuck::cast_slice(&panel_vertices),
        );
        let panel_index_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Panel indices"),
            size: size_of_val(&PANEL_INDICES) as u64,
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&panel_index_buffer, 0, bytemuck::cast_slice(&PANEL_INDICES));

        Ok(Self {
            shadow_pipeline,
            lit_pipelines,
            panel_pipelines,
            tone_pipeline,
            frame_layout,
            camera_bind_group,
            light_bind_group,
            panel_bind_group,
            shadow_bind_group,
            objects_bind_group,
            material_bind_groups,
            camera_buffer,
            vertex_buffer,
            index_buffer,
            panel_vertex_buffer,
            panel_index_buffer,
            shadow_view,
            resolve_texture: None,
            resolve_view: None,
            tone_bind_group: None,
            msaa_color_view: None,
            single_depth_view: None,
            msaa_depth_view: None,
            target_size: (0, 0),
            pending_size: None,
            msaa_enabled: false,
            shadows_enabled: true,
            projection: glam::camera::rh::proj::directx::perspective(
                scene::FOV_Y,
                FALLBACK_SIZE.0 as f32 / FALLBACK_SIZE.1 as f32,
                scene::NEAR,
                scene::FAR,
            ),
        })
    }

    /// Framework contract: called right after `init` and on every resize.
    fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.pending_size = Some(PhysicalSize::new(width, height));
            self.projection = glam::camera::rh::proj::directx::perspective(
                scene::FOV_Y,
                width as f32 / height as f32,
                scene::NEAR,
                scene::FAR,
            );
        }
    }

    fn draw(&mut self, gpu: &Gpu, encoder: &mut CommandEncoder, view: &TextureView) {
        // A pending size (re)creates the HDR kit here, in draw.
        if let Some(size) = self.pending_size.take()
            && self.target_size != (size.width, size.height)
        {
            self.recreate_kit(gpu, size.width, size.height);
        }

        let view_proj = self.projection
            * glam::camera::rh::view::look_at_mat4(scene::EYE, scene::TARGET, Vec3::Y);
        let camera = PassParams {
            view_proj,
            shadows: u32::from(self.shadows_enabled),
        };
        let mut bytes = UniformBuffer::new(Vec::<u8>::new());
        bytes.write(&camera).expect("fits the uniform contract");
        gpu.queue
            .write_buffer(&self.camera_buffer, 0, &bytes.into_inner());

        let mode = usize::from(self.msaa_enabled);

        // Pass 1: shadow depth; only the cube casts.
        if self.shadows_enabled {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Shadow depth pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &self.shadow_view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(1.0),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..RenderPassDescriptor::default()
            });
            pass.set_pipeline(&self.shadow_pipeline);
            pass.set_bind_group(0, &self.light_bind_group, &[]);
            pass.set_bind_group(2, &self.objects_bind_group, &[]);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            let cube: u32 = 0;
            pass.set_immediates(0, bytemuck::bytes_of(&cube));
            pass.draw_indexed(CUBE_MESH.clone(), 0, 0..1);
        }

        // Pass 2: opaque HDR; no resolve yet - the panel still blends into samples.
        {
            let (color_view, depth_view) = if self.msaa_enabled {
                (
                    self.msaa_color_view
                        .as_ref()
                        .expect("msaa kit exists after recreate"),
                    self.msaa_depth_view
                        .as_ref()
                        .expect("msaa kit exists after recreate"),
                )
            } else {
                (
                    self.resolve_view
                        .as_ref()
                        .expect("resolve view exists after recreate"),
                    self.single_depth_view
                        .as_ref()
                        .expect("single kit exists after recreate"),
                )
            };
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Opaque HDR pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: color_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(CLEAR),
                        store: StoreOp::Store,
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
            pass.set_pipeline(&self.lit_pipelines[mode]);
            pass.set_bind_group(0, &self.camera_bind_group, &[]);
            pass.set_bind_group(2, &self.objects_bind_group, &[]);
            // Group 3: the shadow map for the lit fragment stage.
            pass.set_bind_group(3, &self.shadow_bind_group, &[]);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            // Mesh = draw range, material = group, object = immediate index.
            for (object, mesh, material) in
                [(0u32, &CUBE_MESH, 0usize), (1u32, &FLOOR_MESH, 1usize)]
            {
                pass.set_bind_group(1, &self.material_bind_groups[material], &[]);
                pass.set_immediates(0, bytemuck::bytes_of(&object));
                pass.draw_indexed(mesh.clone(), 0, 0..1);
            }
        }

        // Pass 3: transparent panel; depth writes off, MSAA resolves here.
        {
            let resolve_view = self
                .resolve_view
                .as_ref()
                .expect("resolve view exists after recreate");
            let (color_view, depth_view, resolve_target) = if self.msaa_enabled {
                (
                    self.msaa_color_view
                        .as_ref()
                        .expect("msaa kit exists after recreate"),
                    self.msaa_depth_view
                        .as_ref()
                        .expect("msaa kit exists after recreate"),
                    Some(resolve_view),
                )
            } else {
                (
                    resolve_view,
                    self.single_depth_view
                        .as_ref()
                        .expect("single depth exists after recreate"),
                    None,
                )
            };
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Transparent HDR pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: color_view,
                    resolve_target,
                    ops: Operations {
                        load: LoadOp::Load,
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
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..RenderPassDescriptor::default()
            });
            pass.set_pipeline(&self.panel_pipelines[mode]);
            pass.set_bind_group(0, &self.panel_bind_group, &[]);
            pass.set_vertex_buffer(0, self.panel_vertex_buffer.slice(..));
            pass.set_index_buffer(self.panel_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            pass.draw_indexed(0..PANEL_INDICES.len() as u32, 0, 0..1);
        }

        // Pass 4: tone map the HDR frame onto the surface.
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Tone map to surface pass"),
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
                    KeyCode::KeyM => {
                        self.msaa_enabled = !self.msaa_enabled;
                        window.request_redraw();
                    }
                    KeyCode::KeyH => {
                        self.shadows_enabled = !self.shadows_enabled;
                        window.request_redraw();
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

impl FullFrame {
    /// Recreates the HDR attachment kit; the shadow map is fixed-size.
    fn recreate_kit(&mut self, gpu: &Gpu, width: u32, height: u32) {
        let size = Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let resolve = gpu.device.create_texture(&TextureDescriptor {
            label: Some("Single-sample HDR frame"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba16Float,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let resolve_view = resolve.create_view(&TextureViewDescriptor {
            label: Some("Single-sample HDR frame view"),
            ..TextureViewDescriptor::default()
        });
        let tone_bind_group = gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Tone bind group"),
            layout: &self.frame_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&resolve_view),
            }],
        });
        let msaa_color = gpu.device.create_texture(&TextureDescriptor {
            label: Some("MSAA HDR color"),
            size,
            mip_level_count: 1,
            sample_count: SAMPLE_COUNTS[1],
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba16Float,
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let msaa_color_view = msaa_color.create_view(&TextureViewDescriptor {
            label: Some("MSAA HDR color view"),
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
        self.resolve_texture = Some(resolve);
        self.resolve_view = Some(resolve_view);
        self.tone_bind_group = Some(tone_bind_group);
        self.msaa_color_view = Some(msaa_color_view);
        self.single_depth_view = Some(single_depth_view);
        self.msaa_depth_view = Some(msaa_depth_view);
        self.target_size = (width, height);
    }

    /// The verification crate drives this directly.
    pub fn set_msaa(&mut self, enabled: bool) {
        self.msaa_enabled = enabled;
    }

    /// The verification crate drives this directly.
    pub fn set_shadows(&mut self, enabled: bool) {
        self.shadows_enabled = enabled;
    }

    /// The current switches, for diagnostics.
    pub fn modes(&self) -> (bool, bool) {
        (self.msaa_enabled, self.shadows_enabled)
    }
}

fn create_shadow_pipeline(
    gpu: &Gpu,
    layout: &PipelineLayout,
    shader: &ShaderModule,
) -> RenderPipeline {
    gpu.device
        .create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Shadow depth pipeline"),
            layout: Some(layout),
            vertex: VertexState {
                module: shader,
                entry_point: Some("vs_shadow"),
                buffers: &[Some(Vertex::LAYOUT)],
                compilation_options: PipelineCompilationOptions::default(),
            },
            // No fragment stage: depth only.
            fragment: None,
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                front_face: FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                ..PrimitiveState::default()
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            cache: None,
            multiview_mask: None,
        })
}

fn create_lit_pipeline(
    gpu: &Gpu,
    layout: &PipelineLayout,
    shader: &ShaderModule,
    sample_count: u32,
) -> RenderPipeline {
    gpu.device
        .create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Lit HDR pipeline"),
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
                cull_mode: Some(wgpu::Face::Back),
                ..PrimitiveState::default()
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: sample_count,
                ..Default::default()
            },
            cache: None,
            multiview_mask: None,
        })
}

fn create_panel_pipeline(
    gpu: &Gpu,
    layout: &PipelineLayout,
    shader: &ShaderModule,
    sample_count: u32,
) -> RenderPipeline {
    gpu.device
        .create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Panel pipeline"),
            layout: Some(layout),
            vertex: VertexState {
                module: shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(PanelVertex::LAYOUT)],
                compilation_options: PipelineCompilationOptions::default(),
            },
            fragment: Some(FragmentState {
                module: shader,
                entry_point: Some("fs_main"),
                // The blend runs in linear light, before tone mapping.
                targets: &[Some(ColorTargetState {
                    format: TextureFormat::Rgba16Float,
                    blend: Some(STRAIGHT),
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
            // Depth test on, writes off: the panel never occludes anything.
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: Some(false),
                depth_compare: Some(CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: sample_count,
                ..Default::default()
            },
            cache: None,
            multiview_mask: None,
        })
}

fn create_tone_pipeline(
    gpu: &Gpu,
    layout: &PipelineLayout,
    shader: &ShaderModule,
) -> RenderPipeline {
    gpu.device
        .create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Tone map pipeline"),
            layout: Some(layout),
            vertex: VertexState {
                module: shader,
                entry_point: Some("vs_full"),
                buffers: &[],
                compilation_options: PipelineCompilationOptions::default(),
            },
            fragment: Some(FragmentState {
                module: shader,
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
        })
}
