//! Scene pipeline creation.

use crate::mesh::Vertex;
use framework::Gpu;
use wgpu::{
    ColorTargetState, ColorWrites, CompareFunction, DepthStencilState, FragmentState, FrontFace,
    MultisampleState, PipelineCompilationOptions, PipelineLayout, PrimitiveState,
    PrimitiveTopology, RenderPipeline, RenderPipelineDescriptor, ShaderModule, TextureFormat,
    VertexState,
};

pub(crate) fn create_scene_pipeline(
    gpu: &Gpu,
    layout: &PipelineLayout,
    shader: &ShaderModule,
    sample_count: u32,
) -> RenderPipeline {
    gpu.device
        .create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("MSAA scene pipeline"),
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
                // Linear target; sRGB encoding happens only in the fullscreen pass.
                targets: &[Some(ColorTargetState {
                    format: TextureFormat::Rgba8Unorm,
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
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            // The pipeline declares the pass's sample count; all else is identical.
            multisample: MultisampleState {
                count: sample_count,
                ..Default::default()
            },
            cache: None,
            multiview_mask: None,
        })
}
