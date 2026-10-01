use shell::{Gpu, Sample};
use std::error::Error;
use wgpu::{
    Color, ColorTargetState, ColorWrites, CommandEncoder, FragmentState, FrontFace, LoadOp,
    Operations, PipelineCompilationOptions, PrimitiveState, PrimitiveTopology,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor,
    StoreOp, TextureView, VertexState, include_wgsl,
};

/// Chapter 06: per-vertex colors interpolated across the triangle.
pub struct VertexColors {
    pipeline: RenderPipeline,
}

impl Sample for VertexColors {
    fn init(gpu: &Gpu) -> Result<Self, Box<dyn Error>> {
        let shader = gpu
            .device
            .create_shader_module(include_wgsl!("shader.wgsl"));
        let pipeline = gpu
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Vertex colors pipeline"),
                layout: None,
                vertex: VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
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
        Ok(Self { pipeline })
    }

    fn draw(&mut self, _gpu: &Gpu, encoder: &mut CommandEncoder, view: &TextureView) {
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Vertex colors pass"),
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
        pass.draw(0..3, 0..1);
    }
}
