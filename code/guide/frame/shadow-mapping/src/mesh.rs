//! Scene mesh: shared vertices, index ranges per object.

use bytemuck::Pod;
use bytemuck::Zeroable;
use std::ops::Range;
use wgpu::{VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct Vertex {
    /// Position with w = 1: a point.
    pub(crate) position: [f32; 4],
    /// Normal with w = 0; padded to four components for alignment.
    pub(crate) normal: [f32; 4],
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

/// The +Y normal of the floor quad, repeated per corner.
pub(crate) const FLOOR_NORMAL: [f32; 4] = [0.0, 1.0, 0.0, 0.0];

/// Floor receiver and cube caster in one indexed mesh; the cube center
/// sits at (0, 0.51, 0) so the shadow keeps a contact region.
pub(crate) const VERTICES: [Vertex; 28] = [
    // Floor: 4x4 m quad at y = 0, as seen from above.
    Vertex {
        position: [-2.0, 0.0, -2.0, 1.0],
        normal: FLOOR_NORMAL,
    },
    Vertex {
        position: [2.0, 0.0, -2.0, 1.0],
        normal: FLOOR_NORMAL,
    },
    Vertex {
        position: [-2.0, 0.0, 2.0, 1.0],
        normal: FLOOR_NORMAL,
    },
    Vertex {
        position: [2.0, 0.0, 2.0, 1.0],
        normal: FLOOR_NORMAL,
    },
    // Cube +Z face.
    Vertex {
        position: [-0.5, 1.01, 0.5, 1.0],
        normal: [0.0, 0.0, 1.0, 0.0],
    },
    Vertex {
        position: [0.5, 1.01, 0.5, 1.0],
        normal: [0.0, 0.0, 1.0, 0.0],
    },
    Vertex {
        position: [-0.5, 0.01, 0.5, 1.0],
        normal: [0.0, 0.0, 1.0, 0.0],
    },
    Vertex {
        position: [0.5, 0.01, 0.5, 1.0],
        normal: [0.0, 0.0, 1.0, 0.0],
    },
    // Cube -Z face.
    Vertex {
        position: [0.5, 1.01, -0.5, 1.0],
        normal: [0.0, 0.0, -1.0, 0.0],
    },
    Vertex {
        position: [-0.5, 1.01, -0.5, 1.0],
        normal: [0.0, 0.0, -1.0, 0.0],
    },
    Vertex {
        position: [0.5, 0.01, -0.5, 1.0],
        normal: [0.0, 0.0, -1.0, 0.0],
    },
    Vertex {
        position: [-0.5, 0.01, -0.5, 1.0],
        normal: [0.0, 0.0, -1.0, 0.0],
    },
    // Cube +X face.
    Vertex {
        position: [0.5, 1.01, 0.5, 1.0],
        normal: [1.0, 0.0, 0.0, 0.0],
    },
    Vertex {
        position: [0.5, 1.01, -0.5, 1.0],
        normal: [1.0, 0.0, 0.0, 0.0],
    },
    Vertex {
        position: [0.5, 0.01, 0.5, 1.0],
        normal: [1.0, 0.0, 0.0, 0.0],
    },
    Vertex {
        position: [0.5, 0.01, -0.5, 1.0],
        normal: [1.0, 0.0, 0.0, 0.0],
    },
    // Cube -X face.
    Vertex {
        position: [-0.5, 1.01, -0.5, 1.0],
        normal: [-1.0, 0.0, 0.0, 0.0],
    },
    Vertex {
        position: [-0.5, 1.01, 0.5, 1.0],
        normal: [-1.0, 0.0, 0.0, 0.0],
    },
    Vertex {
        position: [-0.5, 0.01, -0.5, 1.0],
        normal: [-1.0, 0.0, 0.0, 0.0],
    },
    Vertex {
        position: [-0.5, 0.01, 0.5, 1.0],
        normal: [-1.0, 0.0, 0.0, 0.0],
    },
    // Cube +Y face (top).
    Vertex {
        position: [0.5, 1.01, 0.5, 1.0],
        normal: [0.0, 1.0, 0.0, 0.0],
    },
    Vertex {
        position: [-0.5, 1.01, 0.5, 1.0],
        normal: [0.0, 1.0, 0.0, 0.0],
    },
    Vertex {
        position: [0.5, 1.01, -0.5, 1.0],
        normal: [0.0, 1.0, 0.0, 0.0],
    },
    Vertex {
        position: [-0.5, 1.01, -0.5, 1.0],
        normal: [0.0, 1.0, 0.0, 0.0],
    },
    // Cube -Y face (bottom).
    Vertex {
        position: [-0.5, 0.01, 0.5, 1.0],
        normal: [0.0, -1.0, 0.0, 0.0],
    },
    Vertex {
        position: [0.5, 0.01, 0.5, 1.0],
        normal: [0.0, -1.0, 0.0, 0.0],
    },
    Vertex {
        position: [-0.5, 0.01, -0.5, 1.0],
        normal: [0.0, -1.0, 0.0, 0.0],
    },
    Vertex {
        position: [0.5, 0.01, -0.5, 1.0],
        normal: [0.0, -1.0, 0.0, 0.0],
    },
];

/// Two triangles per quad, the shared-diagonal pattern of the guide.
pub(crate) const INDICES: [u16; 42] = [
    0, 1, 2, 2, 1, 3, 4, 5, 6, 6, 5, 7, 8, 9, 10, 10, 9, 11, 12, 13, 14, 14, 13, 15, 16, 17, 18,
    18, 17, 19, 20, 21, 22, 22, 21, 23, 24, 25, 26, 26, 25, 27,
];

/// Draw ranges inside INDICES: one mesh, the passes pick their parts.
pub(crate) const FLOOR_INDICES: Range<u32> = 0..6;
pub(crate) const CUBE_INDICES: Range<u32> = 6..42;
