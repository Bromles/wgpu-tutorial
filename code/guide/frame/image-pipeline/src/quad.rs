//! Screen quad mesh and its vertex layouts.

use wgpu::{VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode};
use bytemuck::Pod;
use bytemuck::Zeroable;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct Vertex {
    pub(crate) position: [f32; 4],
}

pub(crate) const VERTICES: [Vertex; 4] = [
    Vertex {
        position: [-0.75, 0.75, 0.5, 1.0],
    },
    Vertex {
        position: [0.75, 0.75, 0.5, 1.0],
    },
    Vertex {
        position: [-0.75, -0.75, 0.5, 1.0],
    },
    Vertex {
        position: [0.75, -0.75, 0.5, 1.0],
    },
];

/// v grows downward on screen, matching the top-to-bottom upload order.
pub(crate) const UVS: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];

pub(crate) const INDICES: [u16; 6] = [0, 1, 2, 2, 1, 3];

impl Vertex {
    pub(crate) const LAYOUT: VertexBufferLayout<'static> = VertexBufferLayout {
        array_stride: 16,
        step_mode: VertexStepMode::Vertex,
        attributes: &[VertexAttribute {
            format: VertexFormat::Float32x4,
            offset: 0,
            shader_location: 0,
        }],
    };
}

pub(crate) const UV_LAYOUT: VertexBufferLayout<'static> = VertexBufferLayout {
    array_stride: 8,
    step_mode: VertexStepMode::Vertex,
    attributes: &[VertexAttribute {
        format: VertexFormat::Float32x2,
        offset: 0,
        shader_location: 2,
    }],
};
