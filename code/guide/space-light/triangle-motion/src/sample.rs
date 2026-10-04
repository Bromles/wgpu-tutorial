use encase::UniformBuffer;
use framework::{Gpu, Sample};
use std::error::Error;
use std::time::Instant;
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

use crate::params::Params;
use glam::Vec2;

/// Horizontal speed of the point, clip units per second.
const SPEED: f32 = 0.2;
/// Rotation speed, radians per second.
const ANGULAR_SPEED: f32 = 0.5;
/// The point ping-pongs inside [-0.6, 0.6] along X.
const TRAVEL: f32 = 0.6;

/// Triangular wave: +TRAVEL at t=0, -TRAVEL at the half period, fold-back
/// without the modulo jump (unit tests pin the continuity).
pub fn ping_pong_x(elapsed: f32) -> f32 {
    let distance = (SPEED * elapsed) % (4.0 * TRAVEL);
    (distance - 2.0 * TRAVEL).abs() - TRAVEL
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 4],
    color: [f32; 4],
}

// An asymmetric triangle: swapping any two vertices would change the shape.
const VERTICES: [Vertex; 3] = [
    Vertex {
        position: [-0.25, -0.2, 0.5, 1.0],
        color: [1.0, 0.0, 0.0, 1.0],
    },
    Vertex {
        position: [0.35, -0.15, 0.5, 1.0],
        color: [0.0, 1.0, 0.0, 1.0],
    },
    Vertex {
        position: [0.0, 0.3, 0.5, 1.0],
        color: [0.0, 0.0, 1.0, 1.0],
    },
];

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

/// Chapter 15: a point with translation, scale and rotation.
pub struct TriangleMotion {
    pipeline: RenderPipeline,
    bind_group: BindGroup,
    params_buffer: Buffer,
    vertex_buffer: Buffer,
    elapsed: f32,
    paused: bool,
    last_instant: Option<Instant>,
    /// When set, draw uses these instead of the clock; R returns to animation.
    manual_params: Option<Params>,
}

impl Sample for TriangleMotion {
    fn init(gpu: &Gpu) -> Result<Self, Box<dyn Error>> {
        let shader = gpu
            .device
            .create_shader_module(include_wgsl!("shader.wgsl"));
        let layout = gpu
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Motion layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(16),
                    },
                    count: None,
                }],
            });
        let pipeline_layout = gpu
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Motion pipeline layout"),
                bind_group_layouts: &[Some(&layout)],
                immediate_size: 0,
            });
        let pipeline = gpu
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Motion pipeline"),
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
        let params_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Motion params"),
            size: 16,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Motion bind group"),
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
        Ok(Self {
            pipeline,
            bind_group,
            params_buffer,
            vertex_buffer,
            elapsed: 0.0,
            paused: false,
            last_instant: None,
            manual_params: None,
        })
    }

    fn draw(&mut self, gpu: &Gpu, encoder: &mut CommandEncoder, view: &TextureView) {
        let now = Instant::now();
        if let Some(last) = self.last_instant
            && !self.paused
        {
            self.elapsed += now.duration_since(last).as_secs_f32();
        }
        self.last_instant = Some(now);
        let params = match self.manual_params {
            Some(params) => params,
            None => Params::new(
                Vec2::new(ping_pong_x(self.elapsed), 0.0),
                Vec2::new(1.0, ANGULAR_SPEED * self.elapsed),
            ),
        };
        let mut bytes = UniformBuffer::new(Vec::<u8>::new());
        bytes.write(&params).expect("fits the uniform contract");
        gpu.queue
            .write_buffer(&self.params_buffer, 0, &bytes.into_inner());

        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Motion pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view,
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
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.draw(0..3, 0..1);
    }

    fn window_event(&mut self, window: &winit::window::Window, event: &WindowEvent) {
        if let WindowEvent::KeyboardInput {
            event: key_event, ..
        } = event
            && key_event.state == ElementState::Pressed
            && let PhysicalKey::Code(key_code) = key_event.physical_key
        {
            match key_code {
                KeyCode::Space => self.paused = !self.paused,
                KeyCode::KeyR => {
                    self.elapsed = 0.0;
                    self.manual_params = None;
                }
                _ => {}
            }
        }

        if matches!(event, WindowEvent::RedrawRequested) {
            window.request_redraw();
        }
    }
}

impl TriangleMotion {
    /// Places the triangle at explicit parameters; used by the verification.
    pub fn set_params(&mut self, params: Params) {
        self.manual_params = Some(params);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Half period: 2*TRAVEL/SPEED = 6 s from +TRAVEL down to -TRAVEL.
    const HALF_PERIOD: f32 = 2.0 * TRAVEL / SPEED;

    #[test]
    fn starts_at_the_right_edge_and_reaches_the_left() {
        assert!((ping_pong_x(0.0) - TRAVEL).abs() < 1e-6);
        assert!((ping_pong_x(HALF_PERIOD) + TRAVEL).abs() < 1e-6);
    }

    #[test]
    fn position_stays_inside_the_travel_band() {
        for step in 0..2000 {
            let t = step as f32 * 0.01;
            let x = ping_pong_x(t);
            assert!(
                (-TRAVEL - 1e-6..=TRAVEL + 1e-6).contains(&x),
                "x={x} at t={t}"
            );
        }
    }

    #[test]
    fn turnaround_is_continuous_not_a_jump() {
        let dt = 0.01;
        let before = ping_pong_x(HALF_PERIOD - dt);
        let after = ping_pong_x(HALF_PERIOD + dt);
        assert!((after - before).abs() <= 2.0 * SPEED * dt + 1e-6);
        let period = 2.0 * HALF_PERIOD;
        let p_before = ping_pong_x(period - dt);
        let p_after = ping_pong_x(period + dt);
        assert!((p_after - p_before).abs() <= 2.0 * SPEED * dt + 1e-6);
        // The old sawtooth jumped 2*TRAVEL right here.
        assert!((ping_pong_x(period) - TRAVEL).abs() < 1e-6);
    }

    #[test]
    fn speed_is_constant_between_turnarounds() {
        let dt = 0.05;
        for t in [0.0_f32, 1.0, 3.0, 7.0, 9.0] {
            let dx = ping_pong_x(t + dt) - ping_pong_x(t);
            assert!((dx.abs() - SPEED * dt).abs() < 1e-5, "t={t} dx={dx}");
        }
    }
}
