//! Chapter mesh: vertices, builders, index ranges.

use wgpu::{VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode};
use bytemuck::Pod;
use bytemuck::Zeroable;

pub(crate) const RED: [f32; 4] = [0.85, 0.25, 0.25, 1.0];
pub(crate) const BLUE: [f32; 4] = [0.25, 0.4, 0.85, 1.0];

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct Vertex {
    position: [f32; 4],
    color: [f32; 4],
}

impl Vertex {
    pub(crate) const LAYOUT: VertexBufferLayout<'static> = VertexBufferLayout {
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

/// Two triangles crossing along x = 0, z = 0, y in 0.4..2.6, both CCW from the eye.
/// B is tilted around Y (z = 2.8333*x) so it does not project onto a line.
pub(crate) const VERTICES: [Vertex; 6] = [
    Vertex {
        position: [-1.7, 0.1, 0.0, 1.0],
        color: RED,
    },
    Vertex {
        position: [1.7, 0.1, 0.0, 1.0],
        color: RED,
    },
    Vertex {
        position: [0.0, 2.6, 0.0, 1.0],
        color: RED,
    },
    Vertex {
        position: [-0.6, 0.4, -1.7, 1.0],
        color: BLUE,
    },
    Vertex {
        position: [0.6, 0.4, 1.7, 1.0],
        color: BLUE,
    },
    Vertex {
        position: [0.0, 2.8, 0.0, 1.0],
        color: BLUE,
    },
];

/// Swapping the outer blue indices reverses its winding, visible only once culling is on.
pub(crate) const INDICES: [u16; 6] = [0, 1, 2, 3, 4, 5];
pub(crate) const FLIPPED_INDICES: [u16; 6] = [0, 1, 2, 5, 4, 3];
