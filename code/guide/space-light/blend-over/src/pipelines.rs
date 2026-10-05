//! Blend states and pipeline creation.

use crate::mesh::Vertex;
use framework::Gpu;
use wgpu::{
    BlendComponent, BlendFactor, BlendOperation, BlendState, ColorTargetState, ColorWrites,
    FragmentState, FrontFace, PipelineCompilationOptions, PipelineLayout,
    PrimitiveState, PrimitiveTopology, RenderPipeline, RenderPipelineDescriptor, ShaderModule,
    VertexState,
};
use wgpu::MultisampleState;

/// Over for straight sources; components written out field by field.
pub(crate) const STRAIGHT: BlendState = BlendState { color: BlendComponent { src_factor: BlendFactor::SrcAlpha, dst_factor: BlendFactor::OneMinusSrcAlpha, operation: BlendOperation::Add,
    },
    alpha: BlendComponent {
        src_factor: BlendFactor::One,
        dst_factor: BlendFactor::OneMinusSrcAlpha,
        operation: BlendOperation::Add,
    },
};

/// Over for premultiplied sources; the source factor is simply One.
pub(crate) const PREMULTIPLIED: BlendState = BlendState { color: BlendComponent { src_factor: BlendFactor::One, dst_factor: BlendFactor::OneMinusSrcAlpha, operation: BlendOperation::Add,
    },
    alpha: STRAIGHT.alpha,
};

///  Chapter 27a: two diagnostic presets composed with over.
pub(crate) fn create_pipeline( gpu: &Gpu, layout: &PipelineLayout, shader: &ShaderModule, label: &str, fragment_entry: &str, blend: Option<BlendState>,
) -> RenderPipeline {
    gpu.device
        .create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(label),
            layout: Some(layout),
            vertex: VertexState {
                module: shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(Vertex::LAYOUT)],
                compilation_options: PipelineCompilationOptions::default(),
            },
            fragment: Some(FragmentState {
                module: shader,
                entry_point: Some(fragment_entry),
                targets: &[Some(ColorTargetState {
                    format: gpu.format,
                    blend,
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
            multisample: MultisampleState::default(),
            cache: None,
            multiview_mask: None,
        })
}
