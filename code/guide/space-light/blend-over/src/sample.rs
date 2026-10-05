
use framework::{Gpu, Sample};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType,
    Buffer, BufferBinding, BufferBindingType, BufferDescriptor, BufferSize, BufferUsages, Color, CommandEncoder, LoadOp, Operations, PipelineLayoutDescriptor, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, ShaderStages, StoreOp, TextureView, include_wgsl,
};
use winit::event::{ElementState, WindowEvent};
use winit::window::Window;
use crate::params::{Params, ortho};
use bytemuck::cast_slice;

use crate::mesh::{vertices,indices,BACKGROUNDS,SOURCES};
use crate::pipelines::{STRAIGHT,PREMULTIPLIED,create_pipeline};
use std::error::Error;
use encase::UniformBuffer;
use wgpu::BindingResource;
use wgpu::IndexFormat;
use winit::keyboard::PhysicalKey;
use winit::keyboard::KeyCode;


/// M switches straight/premultiplied; the frame must not change.
pub struct BlendOver {
    opaque_pipeline: RenderPipeline,
    straight_pipeline: RenderPipeline,
    premultiplied_pipeline: RenderPipeline,
    bind_group: BindGroup,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    premultiplied: bool,
}

impl Sample for BlendOver {
    fn init(gpu: &Gpu) -> Result<Self, Box<dyn Error>> {
        let vertices = vertices();
        let indices = indices();
        let shader = gpu
            .device
            .create_shader_module(include_wgsl!("shader.wgsl"));
        let layout = gpu
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Blend over layout"),
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
        let pipeline_layout = gpu
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Blend over pipeline layout"),
                bind_group_layouts: &[Some(&layout)],
                immediate_size: 0,
            });
        // One geometry, three pipelines: opaque, straight, premultiplied.
        let opaque_pipeline = create_pipeline(
            gpu,
            &pipeline_layout,
            &shader,
            "Blend over pipeline (opaque)",
            "fs_straight",
            None,
        );
        let straight_pipeline = create_pipeline(
            gpu,
            &pipeline_layout,
            &shader,
            "Blend over pipeline (straight)",
            "fs_straight",
            Some(STRAIGHT),
        );
        let premultiplied_pipeline = create_pipeline(
            gpu,
            &pipeline_layout,
            &shader,
            "Blend over pipeline (premultiplied)",
            "fs_premultiplied",
            Some(PREMULTIPLIED),
        );
        let params_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Blend over params"),
            size: 64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        // The camera never moves: the uniform is written once, not per frame.
        let mut bytes = UniformBuffer::new(Vec::<u8>::new());
        bytes
            .write(&Params {
                view_proj: ortho(),
            })
            .expect("fits the uniform contract");
        gpu.queue
            .write_buffer(&params_buffer, 0, &bytes.into_inner());
        let bind_group = gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Blend over bind group"),
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
            label: Some("Preset quads"),
            size: size_of_val(&vertices) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&vertex_buffer, 0, cast_slice(&vertices));
        let index_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Preset quad indices"),
            size: size_of_val(&indices) as u64,
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&index_buffer, 0, cast_slice(&indices));
        Ok(Self {
            opaque_pipeline,
            straight_pipeline,
            premultiplied_pipeline,
            bind_group,
            vertex_buffer,
            index_buffer,
            premultiplied: false,
        })
    }

    fn draw(&mut self,
    _gpu: &Gpu,
    encoder: &mut CommandEncoder,
    view: &TextureView) {
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Blend over pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color {
                        r: 0.08,
                        g: 0.08,
                        b: 0.1,
                        a: 1.0,
                    }),
                    store: StoreOp::Store,
                },
                depth_slice: None,
            })],
            ..RenderPassDescriptor::default()
        });
        pass.set_pipeline(&self.opaque_pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint16);
        // Opaque first: the backgrounds, alpha = 1, no blending.
        pass.draw_indexed(BACKGROUNDS, 0, 0..1);
        // Sources after opaque, in a fixed order.
        let sources = if self.premultiplied {
            &self.premultiplied_pipeline
        } else {
            &self.straight_pipeline
        };
        pass.set_pipeline(sources);
        pass.draw_indexed(SOURCES, 0, 0..1);
    }

    fn window_event(&mut self,
    window: &Window,
    event: &WindowEvent) {
        if let WindowEvent::KeyboardInput {
            event: key_event, ..
        } = event
            && key_event.state == ElementState::Pressed
            && let PhysicalKey::Code(KeyCode::KeyM) = key_event.physical_key
        {
            // Same operator, different representation: the frame must not change.
            self.premultiplied = !self.premultiplied;
            window.request_redraw();
        }
    }
}

impl BlendOver {
    /// Selects the source representation; driven by the verification crate.
    pub fn set_representation(&mut self,
    premultiplied: bool) {
        self.premultiplied = premultiplied;
    }
}
