use std::error::Error;

use encase::UniformBuffer;
use glam::{Mat4, Vec3};
use shell::{Gpu, Sample};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBinding, BufferBindingType, BufferDescriptor,
    BufferSize, BufferUsages, Color, ColorTargetState, ColorWrites, CommandEncoder, FragmentState,
    FrontFace, LoadOp, Operations, PipelineCompilationOptions, PipelineLayout,
    PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology, RenderPassColorAttachment,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, ShaderModule, ShaderStages,
    StoreOp, TextureSampleType, TextureView, TextureViewDimension, VertexAttribute,
    VertexBufferLayout, VertexFormat, VertexState, VertexStepMode, include_wgsl,
};
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::Window;

use crate::params::Params;

/// Camera pose from chapter 18: eye five meters in front of the origin.
const EYE: Vec3 = Vec3::new(0.0, 0.0, 5.0);
/// Vertical field of view of the perspective projection.
const FOV_Y: f32 = std::f32::consts::FRAC_PI_3;
/// Near and far planes shared by both projections, meters from the eye.
const NEAR: f32 = 1.0;
const FAR: f32 = 9.0;
/// Ortho half-height, of the same order as the perspective view near the origin (~5.8 m).
const ORTHO_HALF_HEIGHT: f32 = 3.0;
/// Fallback aspect until the first `Resized` event; matches the default window.
const FALLBACK_ASPECT: f32 = 800.0 / 600.0;

const FLOOR_COLOR: [f32; 4] = [0.35, 0.38, 0.42, 1.0];
const AXIS_X_COLOR: [f32; 4] = [0.85, 0.3, 0.3, 1.0];
const AXIS_Y_COLOR: [f32; 4] = [0.3, 0.7, 0.35, 1.0];
const AXIS_Z_COLOR: [f32; 4] = [0.3, 0.45, 0.85, 1.0];

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 4],
    color: [f32; 4],
    uv: [f32; 2],
}

impl Vertex {
    const LAYOUT: VertexBufferLayout<'static> = VertexBufferLayout {
        array_stride: 40,
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
            VertexAttribute {
                format: VertexFormat::Float32x2,
                offset: 32,
                shader_location: 2,
            },
        ],
    };
}

/// Every object is a quad: CCW corners, two triangles; only the textured quads use uv.
const VERTICES: [Vertex; 24] = [
    // Floor: a small ground plane at y = -1.5, 3.2 x 4 meters.
    Vertex {
        position: [-1.6, -1.5, 2.0, 1.0],
        color: FLOOR_COLOR,
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [1.6, -1.5, 2.0, 1.0],
        color: FLOOR_COLOR,
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [1.6, -1.5, -2.0, 1.0],
        color: FLOOR_COLOR,
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [-1.6, -1.5, -2.0, 1.0],
        color: FLOOR_COLOR,
        uv: [0.0, 0.0],
    },
    // World axes as thin strips in the z = 0 plane: +X to the right...
    Vertex {
        position: [0.0, -0.02, 0.0, 1.0],
        color: AXIS_X_COLOR,
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [2.2, -0.02, 0.0, 1.0],
        color: AXIS_X_COLOR,
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [2.2, 0.02, 0.0, 1.0],
        color: AXIS_X_COLOR,
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [0.0, 0.02, 0.0, 1.0],
        color: AXIS_X_COLOR,
        uv: [0.0, 0.0],
    },
    // ...+Y up...
    Vertex {
        position: [-0.02, 0.0, 0.0, 1.0],
        color: AXIS_Y_COLOR,
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [0.02, 0.0, 0.0, 1.0],
        color: AXIS_Y_COLOR,
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [0.02, 2.2, 0.0, 1.0],
        color: AXIS_Y_COLOR,
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [-0.02, 2.2, 0.0, 1.0],
        color: AXIS_Y_COLOR,
        uv: [0.0, 0.0],
    },
    // ...and +Z toward the camera, lifted by its half thickness so it is not seen edge-on.
    Vertex {
        position: [-0.02, 0.05, 0.0, 1.0],
        color: AXIS_Z_COLOR,
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [0.02, 0.05, 0.0, 1.0],
        color: AXIS_Z_COLOR,
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [0.02, 0.05, 2.0, 1.0],
        color: AXIS_Z_COLOR,
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [-0.02, 0.05, 2.0, 1.0],
        color: AXIS_Z_COLOR,
        uv: [0.0, 0.0],
    },
    // Far quad: 0.8 x 0.8 m textured square in the z = 1 plane, 4 m from the eye.
    Vertex {
        position: [-0.4, -1.0, 1.0, 1.0],
        color: [0.0; 4],
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [0.4, -1.0, 1.0, 1.0],
        color: [0.0; 4],
        uv: [1.0, 0.0],
    },
    Vertex {
        position: [0.4, -0.2, 1.0, 1.0],
        color: [0.0; 4],
        uv: [1.0, 1.0],
    },
    Vertex {
        position: [-0.4, -0.2, 1.0, 1.0],
        color: [0.0; 4],
        uv: [0.0, 1.0],
    },
    // Near quad: the same size in the z = 3 plane, 2 m from the eye.
    Vertex {
        position: [-0.4, 0.2, 3.0, 1.0],
        color: [0.0; 4],
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [0.4, 0.2, 3.0, 1.0],
        color: [0.0; 4],
        uv: [1.0, 0.0],
    },
    Vertex {
        position: [0.4, 1.0, 3.0, 1.0],
        color: [0.0; 4],
        uv: [1.0, 1.0],
    },
    Vertex {
        position: [-0.4, 1.0, 3.0, 1.0],
        color: [0.0; 4],
        uv: [0.0, 1.0],
    },
];

const INDICES: [u16; 36] = [
    0, 1, 2, 0, 2, 3, // floor
    4, 5, 6, 4, 6, 7, // +X axis
    8, 9, 10, 8, 10, 11, // +Y axis
    12, 13, 14, 12, 14, 15, // +Z axis
    16, 17, 18, 16, 18, 19, // far quad
    20, 21, 22, 20, 22, 23, // near quad
];

/// Which projection the frame uses; the P key switches it.
#[derive(Clone, Copy, PartialEq)]
pub enum Projection {
    Perspective,
    Orthographic,
}

/// Chapter 19a: two equal textured quads at 2 and 4 m, shown through a switchable projection.
pub struct OrthoPerspective {
    flat_pipeline: RenderPipeline,
    textured_pipeline: RenderPipeline,
    bind_group: BindGroup,
    params_buffer: Buffer,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    projection: Projection,
    aspect: f32,
    /// Nonzero target size seen in the events; consumed by the next draw.
    pending_size: Option<PhysicalSize<u32>>,
}

impl Sample for OrthoPerspective {
    fn init(gpu: &Gpu) -> Result<Self, Box<dyn Error>> {
        let shader = gpu
            .device
            .create_shader_module(include_wgsl!("shader.wgsl"));
        let layout = gpu
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Ortho perspective layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(128),
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
        let pipeline_layout = gpu
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Ortho perspective pipeline layout"),
                bind_group_layouts: &[Some(&layout)],
                immediate_size: 0,
            });
        let flat_pipeline = create_pipeline(
            gpu,
            &pipeline_layout,
            &shader,
            "Flat pipeline",
            "vs_flat",
            "fs_flat",
        );
        let textured_pipeline = create_pipeline(
            gpu,
            &pipeline_layout,
            &shader,
            "Textured pipeline",
            "vs_textured",
            "fs_textured",
        );
        let params_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Ortho perspective params"),
            size: 128,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let (_texture, texture_view) = crate::texture::create(gpu);
        let bind_group = gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Ortho perspective bind group"),
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
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
            ],
        });
        let vertex_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Scene vertices"),
            size: size_of_val(&VERTICES) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&VERTICES));
        let index_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Scene indices"),
            size: size_of_val(&INDICES) as u64,
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&index_buffer, 0, bytemuck::cast_slice(&INDICES));
        Ok(Self {
            flat_pipeline,
            textured_pipeline,
            bind_group,
            params_buffer,
            vertex_buffer,
            index_buffer,
            projection: Projection::Perspective,
            aspect: FALLBACK_ASPECT,
            pending_size: None,
        })
    }

    /// Called once after init and on every resize; the offscreen harness calls it before its draw.
    fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.pending_size = Some(PhysicalSize::new(width, height));
        }
    }

    fn draw(&mut self, gpu: &Gpu, encoder: &mut CommandEncoder, view: &TextureView) {
        if let Some(size) = self.pending_size.take() {
            self.aspect = size.width as f32 / size.height as f32;
        }
        let params = Params {
            view: glam::camera::rh::view::look_at_mat4(EYE, Vec3::ZERO, Vec3::Y),
            proj: self.projection_matrix(),
        };
        let mut bytes = UniformBuffer::new(Vec::<u8>::new());
        bytes.write(&params).expect("fits the uniform contract");
        gpu.queue
            .write_buffer(&self.params_buffer, 0, &bytes.into_inner());

        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Ortho perspective pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color {
                        r: 0.08,
                        g: 0.09,
                        b: 0.12,
                        a: 1.0,
                    }),
                    store: StoreOp::Store,
                },
                depth_slice: None,
            })],
            ..RenderPassDescriptor::default()
        });
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        // Depth testing arrives in chapter 20: the painter's order is the whole story.
        pass.set_pipeline(&self.flat_pipeline);
        pass.draw_indexed(0..24, 0, 0..1);
        pass.set_pipeline(&self.textured_pipeline);
        pass.draw_indexed(24..30, 0, 0..1);
        pass.draw_indexed(30..36, 0, 0..1);
    }

    fn window_event(&mut self, window: &Window, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput {
                event: key_event, ..
            } if key_event.state == ElementState::Pressed
                && let PhysicalKey::Code(KeyCode::KeyP) = key_event.physical_key =>
            {
                self.projection = match self.projection {
                    Projection::Perspective => Projection::Orthographic,
                    Projection::Orthographic => Projection::Perspective,
                };
                // The frame is static: repaint exactly when the key changed it.
                window.request_redraw();
            }
            _ => {}
        }
    }
}

impl OrthoPerspective {
    /// Current mode's projection: perspective divides by distance, ortho maps meters to clip.
    fn projection_matrix(&self) -> Mat4 {
        match self.projection {
            Projection::Perspective => {
                glam::camera::rh::proj::directx::perspective(FOV_Y, self.aspect, NEAR, FAR)
            }
            Projection::Orthographic => {
                let half_width = ORTHO_HALF_HEIGHT * self.aspect;
                glam::camera::rh::proj::directx::orthographic(
                    -half_width,
                    half_width,
                    -ORTHO_HALF_HEIGHT,
                    ORTHO_HALF_HEIGHT,
                    NEAR,
                    FAR,
                )
            }
        }
    }

    /// Shows the given projection; the verification crate drives this directly.
    pub fn set_projection(&mut self, projection: Projection) {
        self.projection = projection;
    }
}

fn create_pipeline(
    gpu: &Gpu,
    layout: &PipelineLayout,
    shader: &ShaderModule,
    label: &str,
    vertex_entry: &str,
    fragment_entry: &str,
) -> RenderPipeline {
    gpu.device
        .create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(label),
            layout: Some(layout),
            vertex: VertexState {
                module: shader,
                entry_point: Some(vertex_entry),
                buffers: &[Some(Vertex::LAYOUT)],
                compilation_options: PipelineCompilationOptions::default(),
            },
            fragment: Some(FragmentState {
                module: shader,
                entry_point: Some(fragment_entry),
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
