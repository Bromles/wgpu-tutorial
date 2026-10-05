use bytemuck::cast_slice;
use framework::{Gpu, Sample};
use glam::camera::rh::view::look_at_mat4;
use std::f32::consts::FRAC_PI_3;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBinding, BufferBindingType, BufferDescriptor,
    BufferSize, BufferUsages, Color, ColorTargetState, ColorWrites, CommandEncoder,
    CompareFunction, DepthStencilState, Extent3d, Face, FragmentState, FrontFace, LoadOp,
    Operations, PipelineCompilationOptions, PipelineLayout, PipelineLayoutDescriptor,
    PrimitiveState, PrimitiveTopology, RenderPassColorAttachment, RenderPassDepthStencilAttachment,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, ShaderModule, ShaderStages,
    StoreOp, Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    TextureView, TextureViewDescriptor, VertexState, include_wgsl,
};
use winit::event::{ElementState, WindowEvent};

use crate::mesh::{FLIPPED_INDICES, INDICES, VERTICES, Vertex};
use crate::params::Params;
use encase::UniformBuffer;
use glam::Vec3;
use glam::camera::rh::proj::directx::perspective;
use std::error::Error;
use wgpu::BindingResource;
use wgpu::IndexFormat;
use wgpu::MultisampleState;
use winit::dpi::PhysicalSize;
use winit::keyboard::KeyCode;
use winit::keyboard::PhysicalKey;
use winit::window::Window;

/// Camera pose from chapter 18: eye five meters in front of the origin.
const EYE: Vec3 = Vec3::new(0.0, 0.0, 5.0);
/// The projection of chapter 19a: 60 degrees vertical, 4:3 frame.
const FOV_Y: f32 = FRAC_PI_3;
const ASPECT: f32 = 800.0 / 600.0;
const NEAR: f32 = 1.0;
const FAR: f32 = 9.0;

/// Culling cycle: the C key walks Off -> Front -> Back -> Off.
#[derive(Clone, Copy, PartialEq)]
pub enum Culling {
    Off,
    Front,
    Back,
}

/// Draw order of the two triangles: the O key swaps it.
#[derive(Clone, Copy, PartialEq)]
pub enum Order {
    RedFirst,
    BlueFirst,
}

/// Chapter 20: winding, culling and depth decide visibility, so draw order stops mattering.
pub struct DepthCulling {
    /// Indexed `[culling][depth]`: both live in the pipeline, so six pipeline objects.
    pipelines: [[RenderPipeline; 2]; 3],
    bind_group: BindGroup,
    params_buffer: Buffer,
    vertex_buffer: Buffer,
    index_buffers: [Buffer; 2],
    culling: Culling,
    depth_enabled: bool,
    order: Order,
    flipped_winding: bool,
    depth_texture: Option<Texture>,
    depth_view: Option<TextureView>,
    depth_size: (u32, u32),
    /// Target aspect, updated with the depth attachment (projection follows the window).
    aspect: f32,
    /// Nonzero target size seen in the events; consumed by the next draw.
    pending_size: Option<PhysicalSize<u32>>,
}

impl Sample for DepthCulling {
    fn init(gpu: &Gpu) -> Result<Self, Box<dyn Error>> {
        let shader = gpu
            .device
            .create_shader_module(include_wgsl!("shader.wgsl"));
        let layout = gpu
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Depth culling layout"),
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
        let pipeline_layout = gpu
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Depth culling pipeline layout"),
                bind_group_layouts: &[Some(&layout)],
                immediate_size: 0,
            });
        let cull_modes = [None, Some(Face::Front), Some(Face::Back)];
        let pipelines = cull_modes.map(|cull| {
            [false, true].map(|depth| create_pipeline(gpu, &pipeline_layout, &shader, cull, depth))
        });
        let params_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Depth culling params"),
            size: 128,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Depth culling bind group"),
            layout: &layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(BufferBinding {
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
            .write_buffer(&vertex_buffer, 0, cast_slice(&VERTICES));
        let index_buffers = [INDICES, FLIPPED_INDICES].map(|indices| {
            let buffer = gpu.device.create_buffer(&BufferDescriptor {
                label: Some("Triangle indices"),
                size: size_of_val(&indices) as u64,
                usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            gpu.queue.write_buffer(&buffer, 0, cast_slice(&indices));
            buffer
        });
        Ok(Self {
            pipelines,
            bind_group,
            params_buffer,
            vertex_buffer,
            index_buffers,
            culling: Culling::Off,
            depth_enabled: true,
            order: Order::RedFirst,
            flipped_winding: false,
            depth_texture: None,
            depth_view: None,
            depth_size: (0, 0),
            aspect: ASPECT,
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
        if let Some(size) = self.pending_size.take()
            && self.depth_size != (size.width, size.height)
        {
            self.recreate_depth(gpu, size.width, size.height);
        }

        let params = Params {
            view: look_at_mat4(EYE, Vec3::ZERO, Vec3::Y),
            proj: perspective(FOV_Y, self.aspect, NEAR, FAR),
        };
        let mut bytes = UniformBuffer::new(Vec::<u8>::new());
        bytes.write(&params).expect("fits the uniform contract");
        gpu.queue
            .write_buffer(&self.params_buffer, 0, &bytes.into_inner());

        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Depth culling pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color {
                        r: 0.1,
                        g: 0.1,
                        b: 0.14,
                        a: 1.0,
                    }),
                    store: StoreOp::Store,
                },
                depth_slice: None,
            })],
            // Cleared to 1.0: the "farther than everything" wall under `Less`.
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
        let cull_index = match self.culling {
            Culling::Off => 0,
            Culling::Front => 1,
            Culling::Back => 2,
        };
        pass.set_pipeline(&self.pipelines[cull_index][usize::from(self.depth_enabled)]);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(
            self.index_buffers[usize::from(self.flipped_winding)].slice(..),
            IndexFormat::Uint16,
        );
        // With depth on, both orders give the same frame; without it, last draw wins.
        let (red, blue) = (0..3u32, 3..6);
        match self.order {
            Order::RedFirst => {
                pass.draw_indexed(red, 0, 0..1);
                pass.draw_indexed(blue, 0, 0..1);
            }
            Order::BlueFirst => {
                pass.draw_indexed(blue, 0, 0..1);
                pass.draw_indexed(red, 0, 0..1);
            }
        }
    }

    fn window_event(&mut self, window: &Window, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput {
                event: key_event, ..
            } if key_event.state == ElementState::Pressed
                && let PhysicalKey::Code(key_code) = key_event.physical_key =>
            {
                // The frame is static: repaint exactly when a key changed it.
                match key_code {
                    KeyCode::KeyC => {
                        self.culling = match self.culling {
                            Culling::Off => Culling::Front,
                            Culling::Front => Culling::Back,
                            Culling::Back => Culling::Off,
                        };
                        window.request_redraw();
                    }
                    KeyCode::KeyD => {
                        self.depth_enabled = !self.depth_enabled;
                        window.request_redraw();
                    }
                    KeyCode::KeyO => {
                        self.order = match self.order {
                            Order::RedFirst => Order::BlueFirst,
                            Order::BlueFirst => Order::RedFirst,
                        };
                        window.request_redraw();
                    }
                    KeyCode::KeyF => {
                        // Swap two indices of the blue triple: the winding experiment.
                        self.flipped_winding = !self.flipped_winding;
                        window.request_redraw();
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

impl DepthCulling {
    /// Recreates the local depth attachment for a new target size.
    fn recreate_depth(&mut self, gpu: &Gpu, width: u32, height: u32) {
        let texture = gpu.device.create_texture(&TextureDescriptor {
            label: Some("Depth culling depth"),
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
        let view = texture.create_view(&TextureViewDescriptor::default());
        self.depth_texture = Some(texture);
        self.depth_view = Some(view);
        self.depth_size = (width, height);
        self.aspect = width as f32 / height as f32;
    }

    /// Shows the given culling mode; driven by the verification crate.
    pub fn set_culling(&mut self, culling: Culling) {
        self.culling = culling;
    }

    /// Enables or disables the depth test; driven by the verification crate.
    pub fn set_depth_enabled(&mut self, depth_enabled: bool) {
        self.depth_enabled = depth_enabled;
    }

    /// Sets the draw order; driven by the verification crate.
    pub fn set_order(&mut self, order: Order) {
        self.order = order;
    }

    /// Reverses the blue triangle's winding; driven by the verification crate.
    pub fn set_flipped_winding(&mut self, flipped_winding: bool) {
        self.flipped_winding = flipped_winding;
    }
}

fn create_pipeline(
    gpu: &Gpu,
    layout: &PipelineLayout,
    shader: &ShaderModule,
    cull: Option<Face>,
    depth: bool,
) -> RenderPipeline {
    gpu.device
        .create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Depth culling pipeline"),
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
                    format: gpu.format,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: PipelineCompilationOptions::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                front_face: FrontFace::Ccw,
                cull_mode: cull,
                ..PrimitiveState::default()
            },
            // The pass always has a depth attachment, so "depth off" must still declare the format.
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: Some(depth),
                depth_compare: Some(if depth {
                    CompareFunction::Less
                } else {
                    CompareFunction::Always
                }),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: MultisampleState::default(),
            cache: None,
            multiview_mask: None,
        })
}
