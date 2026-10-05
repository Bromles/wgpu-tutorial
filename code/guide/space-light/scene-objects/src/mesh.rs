//! Chapter mesh: vertices, builders, index ranges.

use bytemuck::Pod;
use bytemuck::Zeroable;
use wgpu::{VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode};

///  The single mesh: a plane authored around the left position, so object 0 needs no model.
pub(crate) const CENTER_X: f32 = -0.9;
pub(crate) const HALF_X: f32 = 0.8;
pub(crate) const HALF_Y: f32 = 0.6;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct Vertex {
    /// Position with w = 1: a point of the z = 0 plane, around CENTER_X.
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

pub(crate) fn plane_vertices() -> [Vertex; 4] {
    let normal = [0.0, 0.0, 1.0, 0.0];
    [
        Vertex {
            position: [CENTER_X - HALF_X, -HALF_Y, 0.0, 1.0],
            normal,
        },
        Vertex {
            position: [CENTER_X + HALF_X, -HALF_Y, 0.0, 1.0],
            normal,
        },
        Vertex {
            position: [CENTER_X - HALF_X, HALF_Y, 0.0, 1.0],
            normal,
        },
        Vertex {
            position: [CENTER_X + HALF_X, HALF_Y, 0.0, 1.0],
            normal,
        },
    ]
}

// Two triangles over the four corners; both draws read exactly these six indices.
pub(crate) const INDICES: [u16; 6] = [0, 1, 2, 2, 1, 3];
