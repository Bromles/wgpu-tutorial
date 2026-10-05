//! Pipeline creation for the four frame passes.

use framework::Gpu;

use wgpu::{
    BlendComponent, BlendFactor, BlendOperation, BlendState, ColorTargetState, ColorWrites,
    CompareFunction, DepthStencilState, Face, FragmentState, FrontFace, MultisampleState, PipelineCompilationOptions,
    PipelineLayout, PrimitiveState, PrimitiveTopology, RenderPipeline, RenderPipelineDescriptor,
    ShaderModule, TextureFormat, VertexState,
};

use crate::geometry::{PanelVertex,Vertex};

/// Straight-alpha over: RGB weighed by the source alpha itself.
const STRAIGHT: BlendState = BlendState { color: BlendComponent { src_factor: BlendFactor::SrcAlpha, dst_factor: BlendFactor::OneMinusSrcAlpha, operation: BlendOperation::Add,
    },
    alpha: BlendComponent {
        src_factor: BlendFactor::One,
        dst_factor: BlendFactor::OneMinusSrcAlpha,
        operation: BlendOperation::Add,
    },
};

pub(crate) fn create_shadow_pipeline(
    gpu: &Gpu,
    layout: &PipelineLayout,
    shader: &ShaderModule,
) -> RenderPipeline {
    gpu.device
        .create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Shadow depth pipeline"),
            layout: Some(layout),
            vertex: VertexState {
                module: shader,
                entry_point: Some("vs_shadow"),
                buffers: &[Some(Vertex::LAYOUT)],
                compilation_options: PipelineCompilationOptions::default(),
            },
            // No fragment stage: depth only.
            fragment: None,
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                ..PrimitiveState::default()
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: MultisampleState::default(),
            cache: None,
            multiview_mask: None,
        })
}

pub(crate) fn create_lit_pipeline(
    gpu: &Gpu,
    layout: &PipelineLayout,
    shader: &ShaderModule,
    sample_count: u32,
) -> RenderPipeline {
    gpu.device
        .create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Lit HDR pipeline"),
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
                    format: TextureFormat::Rgba16Float,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: PipelineCompilationOptions::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                ..PrimitiveState::default()
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: MultisampleState {
                count: sample_count,
                ..Default::default()
            },
            cache: None,
            multiview_mask: None,
        })
}

pub(crate) fn create_panel_pipeline(
    gpu: &Gpu,
    layout: &PipelineLayout,
    shader: &ShaderModule,
    sample_count: u32,
) -> RenderPipeline {
    gpu.device
        .create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Panel pipeline"),
            layout: Some(layout),
            vertex: VertexState {
                module: shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(PanelVertex::LAYOUT)],
                compilation_options: PipelineCompilationOptions::default(),
            },
            fragment: Some(FragmentState {
                module: shader,
                entry_point: Some("fs_main"),
                // The blend runs in linear light, before tone mapping.
                targets: &[Some(ColorTargetState {
                    format: TextureFormat::Rgba16Float,
                    blend: Some(STRAIGHT),
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
            // Depth test on, writes off: the panel never occludes anything.
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: Some(false),
                depth_compare: Some(CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: MultisampleState {
                count: sample_count,
                ..Default::default()
            },
            cache: None,
            multiview_mask: None,
        })
}

pub(crate) fn create_tone_pipeline(
    gpu: &Gpu,
    layout: &PipelineLayout,
    shader: &ShaderModule,
) -> RenderPipeline {
    gpu.device
        .create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Tone map pipeline"),
            layout: Some(layout),
            vertex: VertexState {
                module: shader,
                entry_point: Some("vs_full"),
                buffers: &[],
                compilation_options: PipelineCompilationOptions::default(),
            },
            fragment: Some(FragmentState {
                module: shader,
                entry_point: Some("fs_full"),
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
            multisample: MultisampleState::default(),
            cache: None,
            multiview_mask: None,
        })
}
