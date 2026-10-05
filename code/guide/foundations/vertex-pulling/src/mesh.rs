//! Chapter mesh: vertices and layouts.

use wgpu::{
    BufferAddress, VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode,
};
use bytemuck::Pod;
use bytemuck::Zeroable;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct Vertex {
    pub(crate) position: [f32; 4],
    color: [f32; 4],
}

pub(crate) const VERTICES: [Vertex; 4] = [
    Vertex {
        position: [-0.75, 0.75, 0.5, 1.0],
        color: [1.0, 0.0, 0.0, 1.0],
    },
    Vertex {
        position: [0.75, 0.75, 0.5, 1.0],
        color: [0.0, 1.0, 0.0, 1.0],
    },
    Vertex {
        position: [-0.75, -0.75, 0.5, 1.0],
        color: [0.0, 0.0, 1.0, 1.0],
    },
    Vertex {
        position: [0.75, -0.75, 0.5, 1.0],
        color: [1.0, 1.0, 1.0, 1.0],
    },
];

pub(crate) const INDICES: [u16; 6] = [0, 1, 2, 2, 1, 3];

impl Vertex {
    pub(crate) const LAYOUT: VertexBufferLayout<'static> = VertexBufferLayout {
        array_stride: size_of::<Vertex>() as BufferAddress,
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

