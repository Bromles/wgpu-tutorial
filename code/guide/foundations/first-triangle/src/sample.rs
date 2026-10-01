use wgpu::{
    Color, ColorTargetState, ColorWrites, CommandEncoder, Device, FragmentState, FrontFace, LoadOp,
    Operations, PipelineCompilationOptions, PrimitiveState, PrimitiveTopology,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor,
    StoreOp, TextureFormat, TextureView, VertexState, include_wgsl,
};

/// Drawing logic of the chapter: one pipeline, one triangle, no bound
/// resources. Owns no window state, so it draws into any matching view.
pub struct Sample {
    pipeline: RenderPipeline,
}

impl Sample {
    pub fn new(device: &Device, format: TextureFormat) -> Self {
        let shader = device.create_shader_module(include_wgsl!("shader.wgsl"));
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("First triangle pipeline"),
            // No bind group layouts: the shader reads no bound resources.
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
                    format,
                    // No blending: the fragment value replaces the attachment.
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: PipelineCompilationOptions::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                front_face: FrontFace::Ccw,
                // No culling: both winding orders stay visible.
                cull_mode: None,
                ..PrimitiveState::default()
            },
            // No depth/stencil attachment and no depth testing.
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            cache: None,
            multiview_mask: None,
        });
        Self { pipeline }
    }

    /// Records the whole frame: clear to the familiar gray background, then
    /// draw one triangle over it. Three vertex invocations, no buffers.
    pub fn draw(&self, encoder: &mut CommandEncoder, view: &TextureView) {
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("First triangle pass"),
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
