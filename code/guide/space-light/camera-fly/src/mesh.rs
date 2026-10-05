//! Chapter mesh: vertices, builders, index ranges.

use wgpu::{VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode};
use bytemuck::Pod;
use bytemuck::Zeroable;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct Vertex {
    position: [f32; 4],
    color: [f32; 4],
}

pub(crate) const FLOOR_COLOR: [f32; 4] = [0.42, 0.45, 0.5, 1.0];
pub(crate) const TRIANGLE_A_COLOR: [f32; 4] = [0.85, 0.25, 0.25, 1.0];
pub(crate) const TRIANGLE_B_COLOR: [f32; 4] = [0.25, 0.4, 0.85, 1.0];

pub(crate) const VERTICES: [Vertex; 12] = [
    // Floor: two triangles covering y = 0 over a 12x12 meter square.
    Vertex {
        position: [-6.0, 0.0, -6.0, 1.0],
        color: FLOOR_COLOR,
    },
    Vertex {
        position: [6.0, 0.0, -6.0, 1.0],
        color: FLOOR_COLOR,
    },
    Vertex {
        position: [6.0, 0.0, 6.0, 1.0],
        color: FLOOR_COLOR,
    },
    Vertex {
        position: [-6.0, 0.0, -6.0, 1.0],
        color: FLOOR_COLOR,
    },
    Vertex {
        position: [6.0, 0.0, 6.0, 1.0],
        color: FLOOR_COLOR,
    },
    Vertex {
        position: [-6.0, 0.0, 6.0, 1.0],
        color: FLOOR_COLOR,
    },
    // The crossing triangles of chapter 20; the depth buffer picks the nearest.
    Vertex {
        position: [-1.7, 0.1, 0.0, 1.0],
        color: TRIANGLE_A_COLOR,
    },
    Vertex {
        position: [1.7, 0.1, 0.0, 1.0],
        color: TRIANGLE_A_COLOR,
    },
    Vertex {
        position: [0.0, 2.6, 0.0, 1.0],
        color: TRIANGLE_A_COLOR,
    },
    Vertex {
        position: [0.0, 0.4, -1.7, 1.0],
        color: TRIANGLE_B_COLOR,
    },
    Vertex {
        position: [0.0, 0.4, 1.7, 1.0],
        color: TRIANGLE_B_COLOR,
    },
    Vertex {
        position: [0.0, 2.8, 0.0, 1.0],
        color: TRIANGLE_B_COLOR,
    },
];

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
