//! HDR plane mesh: vertex data and layout.

use wgpu::{VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode, };
use bytemuck::Pod;
use bytemuck::Zeroable;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct Vertex {
    /// Position with w = 1: a point of the z = 0 plane.
    pub(crate) position: [f32; 4],
    /// Normal with w = 0: the plane normal +Z for every corner.
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

/// Plane half extents; the background stays visible at the edges.
const HALF_X: f32 = 2.0;
const HALF_Y: f32 = 1.5;

pub(crate) fn plane_vertices() -> [Vertex; 4] {
    let normal = [0.0, 0.0, 1.0, 0.0];
    [
        Vertex {
            position: [-HALF_X, -HALF_Y, 0.0, 1.0],
            normal,
        },
        Vertex {
            position: [HALF_X, -HALF_Y, 0.0, 1.0],
            normal,
        },
        Vertex {
            position: [-HALF_X, HALF_Y, 0.0, 1.0],
            normal,
        },
        Vertex {
            position: [HALF_X, HALF_Y, 0.0, 1.0],
            normal,
        },
    ]
}

// Two triangles over the four corners; the shared diagonal keeps the plane flat.
pub(crate) const INDICES: [u16; 6] = [0, 1, 2, 2, 1, 3];
