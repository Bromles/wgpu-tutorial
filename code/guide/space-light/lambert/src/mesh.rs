//! Chapter mesh: vertices, builders, index ranges.

use bytemuck::Pod;
use bytemuck::Zeroable;
use std::f32::consts::FRAC_1_SQRT_2;
use wgpu::{VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct Vertex {
    /// Position with w = 1: a point.
    pub(crate) position: [f32; 4],
    /// Normal with w = 0 (a direction), padded to four components for 16-byte alignment.
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

/// Slanted quads: a 1.2x1.2 XY square rotated +30/-30 degrees around X.
/// Tilt by theta: (x, y*cos, y*sin) + center, normal (0, -sin, cos).
pub(crate) const QUAD_A_NORMAL: [f32; 4] = [0.0, -0.5, 0.8660254, 0.0];
pub(crate) const QUAD_B_NORMAL: [f32; 4] = [0.0, 0.5, 0.8660254, 0.0];
/// Split face: coplanar halves sharing the seam x = 0; the crease is the normal attribute alone.
pub(crate) const SPLIT_LEFT_NORMAL: [f32; 4] = [0.0, -FRAC_1_SQRT_2, FRAC_1_SQRT_2, 0.0];
pub(crate) const SPLIT_RIGHT_NORMAL: [f32; 4] = [0.0, 0.0, 1.0, 0.0];

pub(crate) const VERTICES: [Vertex; 16] = [
    // Quad A: tilt +30 degrees, center (-0.95, 0.55, 0).
    Vertex {
        position: [-1.55, 1.0696, 0.3, 1.0],
        normal: QUAD_A_NORMAL,
    },
    Vertex {
        position: [-0.35, 1.0696, 0.3, 1.0],
        normal: QUAD_A_NORMAL,
    },
    Vertex {
        position: [-1.55, 0.0304, -0.3, 1.0],
        normal: QUAD_A_NORMAL,
    },
    Vertex {
        position: [-0.35, 0.0304, -0.3, 1.0],
        normal: QUAD_A_NORMAL,
    },
    // Quad B: tilt -30 degrees, center (0.95, 0.55, 0).
    Vertex {
        position: [0.35, 1.0696, -0.3, 1.0],
        normal: QUAD_B_NORMAL,
    },
    Vertex {
        position: [1.55, 1.0696, -0.3, 1.0],
        normal: QUAD_B_NORMAL,
    },
    Vertex {
        position: [0.35, 0.0304, 0.3, 1.0],
        normal: QUAD_B_NORMAL,
    },
    Vertex {
        position: [1.55, 0.0304, 0.3, 1.0],
        normal: QUAD_B_NORMAL,
    },
    // Split face, left half: x in [-1, 0], normal tilted -45 degrees.
    Vertex {
        position: [-1.0, -0.3, 0.0, 1.0],
        normal: SPLIT_LEFT_NORMAL,
    },
    Vertex {
        position: [0.0, -0.3, 0.0, 1.0],
        normal: SPLIT_LEFT_NORMAL,
    },
    Vertex {
        position: [-1.0, -1.3, 0.0, 1.0],
        normal: SPLIT_LEFT_NORMAL,
    },
    Vertex {
        position: [0.0, -1.3, 0.0, 1.0],
        normal: SPLIT_LEFT_NORMAL,
    },
    // Split face, right half: x in [0, 1], normal facing the camera.
    Vertex {
        position: [0.0, -0.3, 0.0, 1.0],
        normal: SPLIT_RIGHT_NORMAL,
    },
    Vertex {
        position: [1.0, -0.3, 0.0, 1.0],
        normal: SPLIT_RIGHT_NORMAL,
    },
    Vertex {
        position: [0.0, -1.3, 0.0, 1.0],
        normal: SPLIT_RIGHT_NORMAL,
    },
    Vertex {
        position: [1.0, -1.3, 0.0, 1.0],
        normal: SPLIT_RIGHT_NORMAL,
    },
];

// Two triangles per quad; the shared diagonal keeps each quad flat.
pub(crate) const INDICES: [u16; 24] = [
    0, 1, 2, 2, 1, 3, // quad A
    4, 5, 6, 6, 5, 7, // quad B
    8, 9, 10, 10, 9, 11, // split, left half
    12, 13, 14, 14, 13, 15, // split, right half
];
