#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::f32::consts::FRAC_PI_2;
use std::time::Duration;

use encase::ShaderType;
use glam::{Mat4, Vec3};
use rand::Rng;
use wgpu::util::DeviceExt;
use wgpu::{
    include_wgsl, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, BlendComponent, BlendFactor, BlendOperation, BlendState,
    Buffer, BufferBindingType, BufferDescriptor, BufferUsages, Color, ColorTargetState,
    ColorWrites, CommandEncoder, CompareFunction, ComputePassDescriptor,
    ComputePipelineDescriptor, DepthStencilState, Extent3d, FragmentState, LoadOp,
    MultisampleState, Operations, PipelineCompilationOptions, PipelineLayoutDescriptor,
    PrimitiveState, PrimitiveTopology, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, ShaderStages, StencilState, StoreOp, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages, TextureViewDescriptor, VertexState,
};
use winit::dpi::PhysicalSize;
use winit::keyboard::KeyCode;

use framework::{run, Example, GpuContext, Input};

const NUM_PARTICLES: u32 = 2048;

#[derive(ShaderType, Clone, Copy)]
struct ParticleData {
    pos: Vec3,
    vel: Vec3,
    life: f32,
}

#[derive(ShaderType)]
struct SimParams {
    dt: f32,
    gravity: f32,
}

#[derive(ShaderType)]
struct CameraUniforms {
    view_proj: Mat4,
    camera_right: glam::Vec4,
    camera_up: glam::Vec4,
}

struct Camera {
    position: Vec3,
    yaw: f32,
    pitch: f32,
    speed: f32,
    sensitivity: f32,
}

impl Camera {
    fn new(pos: Vec3, yaw: f32, pitch: f32) -> Self {
        Self {
            position: pos,
            yaw,
            pitch,
            speed: 5.0,
            sensitivity: 0.003,
        }
    }
    fn direction(&self) -> Vec3 {
        Vec3::new(
            -self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            -self.yaw.cos() * self.pitch.cos(),
        )
    }
    fn forward(&self) -> Vec3 {
        Vec3::new(-self.yaw.sin(), 0.0, -self.yaw.cos())
    }
    fn right(&self) -> Vec3 {
        Vec3::new(self.yaw.cos(), 0.0, -self.yaw.sin())
    }
    fn up(&self) -> Vec3 {
        Vec3::Y
    }
    fn view_matrix(&self) -> Mat4 {
        Mat4::look_to_rh(self.position, self.direction(), Vec3::Y)
    }
    fn update(&mut self, dt: f32, input: &Input) {
        if input.mouse_button_pressed(1) {
            let (dx, dy) = input.mouse_delta();
            self.yaw -= dx as f32 * self.sensitivity;
            self.pitch -= dy as f32 * self.sensitivity;
            self.pitch = self.pitch.clamp(-FRAC_PI_2 + 0.01, FRAC_PI_2 - 0.01);
        }
        let mut v = Vec3::ZERO;
        if input.key_pressed(KeyCode::KeyW) {
            v += self.forward();
        }
        if input.key_pressed(KeyCode::KeyS) {
            v -= self.forward();
        }
        if input.key_pressed(KeyCode::KeyD) {
            v += self.right();
        }
        if input.key_pressed(KeyCode::KeyA) {
            v -= self.right();
        }
        if input.key_pressed(KeyCode::Space) {
            v.y += 1.0;
        }
        if input.key_pressed(KeyCode::ShiftLeft) {
            v.y -= 1.0;
        }
        if v.length_squared() > 0.0 {
            self.position += v.normalize() * self.speed * dt;
        }
    }
}

struct ParticlesDemo {
    sim_pipeline: wgpu::ComputePipeline,
    render_pipeline: RenderPipeline,
    particle_buffer: Buffer,
    params_buffer: Buffer,
    sim_bind_group: wgpu::BindGroup,
    render_bind_group: wgpu::BindGroup,
    camera_uniform_buffer: Buffer,
    camera_bind_group: wgpu::BindGroup,
    depth_texture: wgpu::Texture,
    depth_texture_view: wgpu::TextureView,
    camera: Camera,
    spawn_timer: f32,
    spawn_offset: u32,
}

impl ParticlesDemo {
    fn create_depth_texture(ctx: &GpuContext) -> (wgpu::Texture, wgpu::TextureView) {
        let s = &ctx.surface_config;
        let t = ctx.device.create_texture(&TextureDescriptor {
            label: Some("Depth"),
            size: Extent3d {
                width: s.width,
                height: s.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let v = t.create_view(&TextureViewDescriptor::default());
        (t, v)
    }

    fn spawn_particles(buffer: &Buffer, ctx: &GpuContext, count: u32, offset: u32) {
        let mut rng = rand::rng();
        let new_particles: Vec<ParticleData> = (0..count)
            .map(|_| {
                let angle = rng.random_range(0.0..std::f32::consts::TAU);
                let speed = rng.random_range(1.0..4.0);
                let y_vel = rng.random_range(2.0..6.0);
                ParticleData {
                    pos: Vec3::ZERO,
                    vel: Vec3::new(
                        angle.cos() * speed * 0.5,
                        y_vel,
                        angle.sin() * speed * 0.5,
                    ),
                    life: rng.random_range(1.5..3.5),
                }
            })
            .collect();

        let mut data = encase::StorageBuffer::new(Vec::new());
        data.write(&new_particles).unwrap();
        ctx.queue.write_buffer(
            buffer,
            (offset as u64) * ParticleData::min_size().get(),
            &data.into_inner(),
        );
    }
}

impl Example for ParticlesDemo {
    fn init(ctx: &GpuContext) -> Self {
        let sim_shader = ctx
            .device
            .create_shader_module(include_wgsl!("simulate.wgsl"));
        let render_shader = ctx
            .device
            .create_shader_module(include_wgsl!("render.wgsl"));

        let mut rng = rand::rng();
        let initial: Vec<ParticleData> = (0..NUM_PARTICLES)
            .map(|_| {
                let angle = rng.random_range(0.0..std::f32::consts::TAU);
                let speed = rng.random_range(1.0..4.0);
                let y_vel = rng.random_range(2.0..6.0);
                ParticleData {
                    pos: Vec3::ZERO,
                    vel: Vec3::new(
                        angle.cos() * speed * 0.5,
                        y_vel,
                        angle.sin() * speed * 0.5,
                    ),
                    life: rng.random_range(1.5..3.5),
                }
            })
            .collect();

        let mut init_data = encase::StorageBuffer::new(Vec::new());
        init_data.write(&initial).unwrap();

        let particle_buffer = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Particles"),
                contents: &init_data.into_inner(),
                usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            });

        let params_buffer = ctx.device.create_buffer(&BufferDescriptor {
            label: Some("SimParams"),
            size: SimParams::min_size().into(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let sim_bgl = ctx
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("SimBGL"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
        let sim_bind_group = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: Some("SimBG"),
            layout: &sim_bgl,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: particle_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let sim_pipeline = ctx
            .device
            .create_compute_pipeline(&ComputePipelineDescriptor {
                label: Some("SimPipeline"),
                layout: Some(
                    &ctx.device
                        .create_pipeline_layout(&PipelineLayoutDescriptor {
                            label: Some("SimLayout"),
                            bind_group_layouts: &[Some(&sim_bgl)],
                            immediate_size: 0,
                        }),
                ),
                module: &sim_shader,
                entry_point: Some("main"),
                compilation_options: PipelineCompilationOptions::default(),
                cache: None,
            });

        let render_bgl = ctx
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("RenderBGL"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let render_bind_group = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: Some("RenderBG"),
            layout: &render_bgl,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: particle_buffer.as_entire_binding(),
            }],
        });

        let camera_uniform_buffer = ctx.device.create_buffer(&BufferDescriptor {
            label: Some("CameraUB"),
            size: CameraUniforms::min_size().into(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let camera_bgl = ctx
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("CameraBGL"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(CameraUniforms::min_size()),
                    },
                    count: None,
                }],
            });
        let camera_bind_group = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: Some("CameraBG"),
            layout: &camera_bgl,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: camera_uniform_buffer.as_entire_binding(),
            }],
        });

        let render_pipeline = ctx
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("ParticlePipe"),
                layout: Some(
                    &ctx.device
                        .create_pipeline_layout(&PipelineLayoutDescriptor {
                            label: Some("RenderLayout"),
                            bind_group_layouts: &[Some(&render_bgl), Some(&camera_bgl)],
                            immediate_size: 0,
                        }),
                ),
                vertex: VertexState {
                    module: &render_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: PipelineCompilationOptions::default(),
                },
                fragment: Some(FragmentState {
                    module: &render_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(ColorTargetState {
                        format: ctx.surface_format,
                        blend: Some(BlendState {
                            color: BlendComponent {
                                src_factor: BlendFactor::SrcAlpha,
                                dst_factor: BlendFactor::One,
                                operation: BlendOperation::Add,
                            },
                            alpha: BlendComponent {
                                src_factor: BlendFactor::SrcAlpha,
                                dst_factor: BlendFactor::OneMinusSrcAlpha,
                                operation: BlendOperation::Add,
                            },
                        }),
                        write_mask: ColorWrites::ALL,
                    })],
                    compilation_options: PipelineCompilationOptions::default(),
                }),
                primitive: PrimitiveState {
                    topology: PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                depth_stencil: Some(DepthStencilState {
                    format: TextureFormat::Depth32Float,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(CompareFunction::Less),
                    stencil: StencilState::default(),
                    bias: Default::default(),
                }),
                multisample: MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                cache: None,
                multiview_mask: None,
            });

        let (depth_texture, depth_texture_view) = Self::create_depth_texture(ctx);
        let camera = Camera::new(Vec3::new(0.0, 3.0, 8.0), 0.0, -0.2);

        Self {
            sim_pipeline,
            render_pipeline,
            particle_buffer,
            params_buffer,
            sim_bind_group,
            render_bind_group,
            camera_uniform_buffer,
            camera_bind_group,
            depth_texture,
            depth_texture_view,
            camera,
            spawn_timer: 0.0,
            spawn_offset: 0,
        }
    }

    fn resize(&mut self, ctx: &GpuContext, _new_size: PhysicalSize<u32>) {
        let (d, v) = Self::create_depth_texture(ctx);
        self.depth_texture = d;
        self.depth_texture_view = v;
    }

    fn update(&mut self, _ctx: &GpuContext, dt: Duration, input: &Input) {
        self.camera.update(dt.as_secs_f32(), input);
        self.spawn_timer += dt.as_secs_f32();
    }

    fn render(&mut self, ctx: &GpuContext, view: &wgpu::TextureView, encoder: &mut CommandEncoder) {
        if self.spawn_timer > 0.1 {
            self.spawn_timer = 0.0;
            let count = 64u32;
            Self::spawn_particles(&self.particle_buffer, ctx, count, self.spawn_offset);
            self.spawn_offset = (self.spawn_offset + count) % NUM_PARTICLES;
        }

        {
            let mut data = encase::UniformBuffer::new(Vec::new());
            data.write(&SimParams {
                dt: 1.0 / 60.0,
                gravity: 9.8,
            })
            .unwrap();
            ctx.queue
                .write_buffer(&self.params_buffer, 0, &data.into_inner());
        }

        let aspect = ctx.surface_config.width as f32 / ctx.surface_config.height as f32;
        let proj = Mat4::perspective_rh(std::f32::consts::FRAC_PI_4, aspect, 0.1, 100.0);
        let view_mat = self.camera.view_matrix();
        let vp = proj * view_mat;
        {
            let mut d = encase::UniformBuffer::new(Vec::new());
            d.write(&CameraUniforms {
                view_proj: vp,
                camera_right: self.camera.right().extend(0.0),
                camera_up: self.camera.up().extend(0.0),
            })
            .unwrap();
            ctx.queue
                .write_buffer(&self.camera_uniform_buffer, 0, &d.into_inner());
        }

        {
            let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("Simulate"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.sim_pipeline);
            cpass.set_bind_group(0, &self.sim_bind_group, &[]);
            cpass.dispatch_workgroups(NUM_PARTICLES / 256, 1, 1);
        }

        {
            let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Particles"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture_view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(1.0),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            rpass.set_pipeline(&self.render_pipeline);
            rpass.set_bind_group(0, &self.render_bind_group, &[]);
            rpass.set_bind_group(1, &self.camera_bind_group, &[]);
            rpass.draw(0..6, 0..NUM_PARTICLES);
        }
    }
}

fn main() {
    run::<ParticlesDemo>("Particles");
}
