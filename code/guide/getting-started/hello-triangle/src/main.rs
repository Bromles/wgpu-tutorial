#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use framework::{Example, GpuContext, run};
use wgpu::{
    BlendComponent, BlendState, Color, ColorTargetState, ColorWrites, CommandEncoder,
    FragmentState, LoadOp, MultisampleState, Operations, PipelineCompilationOptions, PolygonMode,
    PrimitiveState, PrimitiveTopology, RenderPassColorAttachment, RenderPassDescriptor,
    RenderPipeline, RenderPipelineDescriptor, StoreOp, TextureView, VertexState, include_wgsl,
};

struct Triangle {
    pipeline: RenderPipeline,
}

impl Example for Triangle {
    fn init(ctx: &GpuContext) -> Self {
        let shader_module = ctx
            .device
            .create_shader_module(include_wgsl!("shader.wgsl"));

        let pipeline = ctx
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Main Render Pipeline"),
                layout: None,
                vertex: VertexState {
                    module: &shader_module,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: PipelineCompilationOptions::default(),
                },
                fragment: Some(FragmentState {
                    module: &shader_module,
                    entry_point: Some("fs_main"),
                    targets: &[Some(ColorTargetState {
                        format: ctx.surface_format,
                        blend: Some(BlendState {
                            color: BlendComponent::REPLACE,
                            alpha: BlendComponent::REPLACE,
                        }),
                        write_mask: ColorWrites::ALL,
                    })],
                    compilation_options: PipelineCompilationOptions::default(),
                }),
                primitive: PrimitiveState {
                    topology: PrimitiveTopology::TriangleList,
                    front_face: wgpu::FrontFace::Ccw,
                    polygon_mode: PolygonMode::Fill,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                cache: None,
                multiview_mask: None,
            });

        Self { pipeline }
    }

    fn render(&mut self, _ctx: &GpuContext, view: &TextureView, encoder: &mut CommandEncoder) {
        let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Main Render Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::GREEN),
                    store: StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        rpass.set_pipeline(&self.pipeline);
        rpass.draw(0..3, 0..1);
    }
}

fn main() {
    run::<Triangle>("Hello Triangle");
}
