use std::error::Error;

use encase::{StorageBuffer, UniformBuffer};
use shell::{Gpu, Sample};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBinding, BufferBindingType, BufferDescriptor,
    BufferSize, BufferUsages, Color, ColorTargetState, ColorWrites, CommandEncoder, FragmentState,
    FrontFace, LoadOp, Operations, PipelineCompilationOptions, PipelineLayoutDescriptor,
    PrimitiveState, PrimitiveTopology, RenderPassColorAttachment, RenderPassDescriptor,
    RenderPipeline, RenderPipelineDescriptor, ShaderStages, StoreOp, TextureView, VertexAttribute,
    VertexBufferLayout, VertexFormat, VertexState, VertexStepMode, include_wgsl,
};
use winit::event::{ElementState, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

use crate::camera::{self, CameraParams};
use crate::scene::{self, COOL, OBJECT_COUNT, ObjectRecord, WARM};

/// Matches the default window size until the first `Resized` event.
const FALLBACK_SIZE: (u32, u32) = (800, 600);

/// The single mesh: a plane authored around the left position, so object 0 needs no model.
const CENTER_X: f32 = -0.9;
const HALF_X: f32 = 0.8;
const HALF_Y: f32 = 0.6;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    /// Position with w = 1: a point of the z = 0 plane, around CENTER_X.
    position: [f32; 4],
    /// Normal with w = 0: the plane normal +Z for every corner.
    normal: [f32; 4],
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

fn plane_vertices() -> [Vertex; 4] {
    let normal = [0.0, 0.0, 1.0, 0.0];
    [
        Vertex {
            position: [CENTER_X - HALF_X, -HALF_Y, 0.0, 1.0],
            normal,
        },
        Vertex {
            position: [CENTER_X + HALF_X, -HALF_Y, 0.0, 1.0],
            normal,
        },
        Vertex {
            position: [CENTER_X - HALF_X, HALF_Y, 0.0, 1.0],
            normal,
        },
        Vertex {
            position: [CENTER_X + HALF_X, HALF_Y, 0.0, 1.0],
            normal,
        },
    ]
}

// Two triangles over the four corners; both draws read exactly these six indices.
const INDICES: [u16; 6] = [0, 1, 2, 2, 1, 3];

/// Chapter 28: mesh, material and object as separate roles - one mesh, two materials, two records.
/// Draws pick their object by immediate index; M binds one material, X moves object 1.
pub struct SceneObjects {
    pipeline: RenderPipeline,
    camera_bind_group: BindGroup,
    material_bind_groups: [BindGroup; 2],
    objects_bind_group: BindGroup,
    camera_buffer: Buffer,
    objects_buffer: Buffer,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    objects: [ObjectRecord; OBJECT_COUNT],
    /// Refill the object storage at the next draw; the window layer cannot write buffers.
    objects_pending: bool,
    shared_material: bool,
    object1_moved: bool,
    /// Target aspect; draw rebuilds the full camera chain from it.
    aspect: f32,
}

impl Sample for SceneObjects {
    fn init(gpu: &Gpu) -> Result<Self, Box<dyn Error>> {
        // The immediates path needs its feature and budget checked before the first pipeline.
        if !gpu.device.features().contains(wgpu::Features::IMMEDIATES) {
            return Err("This example requires Features::IMMEDIATES (adapter support varies): the object index travels in the command state".into());
        }
        if gpu.device.limits().max_immediate_size < 4 {
            return Err(
                "This example requires max_immediate_size >= 4: one u32 object index per draw"
                    .into(),
            );
        }
        let vertices = plane_vertices();
        let shader = gpu
            .device
            .create_shader_module(include_wgsl!("shader.wgsl"));
        // Bindings by role: camera per frame, material per material, objects per scene change.
        let camera_layout = gpu
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Camera layout"),
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
                        min_binding_size: BufferSize::new(32),
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
        let pipeline_layout = gpu
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Scene objects pipeline layout"),
                bind_group_layouts: &[
                    Some(&camera_layout),
                    Some(&material_layout),
                    Some(&objects_layout),
                ],
                // The immediate block of this pipeline: one u32.
                immediate_size: 4,
            });
        let pipeline = gpu
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Scene objects pipeline"),
                layout: Some(&pipeline_layout),
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
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                cache: None,
                multiview_mask: None,
            });
        let camera_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Camera uniform"),
            size: 80,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let camera_bind_group = gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Camera bind group"),
            layout: &camera_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(BufferBinding {
                    buffer: &camera_buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });
        // Two material buffers uploaded once; materials change by rebinding, not rewriting.
        let material_bind_groups = [WARM, COOL].map(|material| {
            let buffer = gpu.device.create_buffer(&BufferDescriptor {
                label: Some(if material == WARM {
                    "Material uniform (warm)"
                } else {
                    "Material uniform (cool)"
                }),
                size: 32,
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let mut bytes = UniformBuffer::new(Vec::<u8>::new());
            bytes.write(&material).expect("fits the uniform contract");
            gpu.queue.write_buffer(&buffer, 0, &bytes.into_inner());
            gpu.device.create_bind_group(&BindGroupDescriptor {
                label: Some(if material == WARM {
                    "Material bind group (warm)"
                } else {
                    "Material bind group (cool)"
                }),
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
        // The object table: rewritten only when an object moves.
        let objects = scene::objects();
        let objects_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Object storage"),
            size: (OBJECT_COUNT * size_of::<ObjectRecord>()) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut bytes = StorageBuffer::new(Vec::<u8>::new());
        bytes.write(&objects).expect("fits the storage contract");
        gpu.queue
            .write_buffer(&objects_buffer, 0, &bytes.into_inner());
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
        // The shared mesh: one vertex and index buffer for every draw.
        let vertex_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Plane vertices"),
            size: size_of_val(&vertices) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        let index_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Plane indices"),
            size: size_of_val(&INDICES) as u64,
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&index_buffer, 0, bytemuck::cast_slice(&INDICES));
        Ok(Self {
            pipeline,
            camera_bind_group,
            material_bind_groups,
            objects_bind_group,
            camera_buffer,
            objects_buffer,
            vertex_buffer,
            index_buffer,
            objects,
            objects_pending: false,
            shared_material: false,
            object1_moved: false,
            aspect: FALLBACK_SIZE.0 as f32 / FALLBACK_SIZE.1 as f32,
        })
    }

    /// Called once after init and on every resize; the offscreen harness calls it before its draw.
    fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.aspect = width as f32 / height as f32;
        }
    }

    fn draw(&mut self, gpu: &Gpu, encoder: &mut CommandEncoder, view: &TextureView) {
        if self.objects_pending {
            self.upload_objects(gpu);
            self.objects_pending = false;
        }
        // Role per frame: one camera upload, however many objects follow.
        let params = CameraParams {
            view_proj: camera::view_proj(self.aspect),
            eye: camera::EYE.extend(1.0),
        };
        let mut bytes = UniformBuffer::new(Vec::<u8>::new());
        bytes.write(&params).expect("fits the uniform contract");
        gpu.queue
            .write_buffer(&self.camera_buffer, 0, &bytes.into_inner());

        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Scene objects pass"),
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
            ..RenderPassDescriptor::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.camera_bind_group, &[]);
        pass.set_bind_group(2, &self.objects_bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        for object in 0..OBJECT_COUNT as u32 {
            // Role per material: separate keeps the warm/cool pair, shared binds warm for both.
            let material = usize::from(!self.shared_material && object == 1);
            pass.set_bind_group(1, &self.material_bind_groups[material], &[]);
            // Role per draw: the object index rides in the command state, no buffer written between draws.
            pass.set_immediates(0, bytemuck::bytes_of(&object));
            pass.draw_indexed(0..INDICES.len() as u32, 0, 0..1);
        }
    }

    fn window_event(&mut self, window: &winit::window::Window, event: &WindowEvent) {
        if let WindowEvent::KeyboardInput {
            event: key_event, ..
        } = event
            && key_event.state == ElementState::Pressed
            && let PhysicalKey::Code(key_code) = key_event.physical_key
        {
            // The frame is static: repaint exactly when a key changed it.
            match key_code {
                KeyCode::KeyM => {
                    self.shared_material = !self.shared_material;
                    window.request_redraw();
                }
                KeyCode::KeyX => {
                    self.set_object1_moved(!self.object1_moved);
                    window.request_redraw();
                }
                _ => {}
            }
        }
    }
}

impl SceneObjects {
    /// Refills the object storage from draw, where the queue is available.
    fn upload_objects(&mut self, gpu: &Gpu) {
        let mut bytes = StorageBuffer::new(Vec::<u8>::new());
        bytes
            .write(&self.objects)
            .expect("fits the storage contract");
        gpu.queue
            .write_buffer(&self.objects_buffer, 0, &bytes.into_inner());
    }

    /// Binds one material for both draws (true) or restores the warm/cool pair; M does the same.
    pub fn set_shared_material(&mut self, shared: bool) {
        self.shared_material = shared;
    }

    /// Moves object 1 by rewriting its storage record; X does the same.
    pub fn set_object1_moved(&mut self, moved: bool) {
        let pose = if moved {
            scene::RIGHT_MOVED
        } else {
            scene::RIGHT_HOME
        };
        self.objects[1] = scene::object_record(pose);
        self.objects_pending = true;
        self.object1_moved = moved;
    }
}
