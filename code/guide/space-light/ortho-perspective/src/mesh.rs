//! Chapter mesh: vertices, builders, index ranges.

use wgpu::{VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode};
use bytemuck::Pod;
use bytemuck::Zeroable;

pub(crate) const FLOOR_COLOR: [f32; 4] = [0.35, 0.38, 0.42, 1.0];
pub(crate) const AXIS_X_COLOR: [f32; 4] = [0.85, 0.3, 0.3, 1.0];
pub(crate) const AXIS_Y_COLOR: [f32; 4] = [0.3, 0.7, 0.35, 1.0];
pub(crate) const AXIS_Z_COLOR: [f32; 4] = [0.3, 0.45, 0.85, 1.0];

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct Vertex {
    position: [f32; 4],
    color: [f32; 4],
    uv: [f32; 2],
}

impl Vertex {
    pub(crate) const LAYOUT: VertexBufferLayout<'static> = VertexBufferLayout {
        array_stride: 40,
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
            VertexAttribute {
                format: VertexFormat::Float32x2,
                offset: 32,
                shader_location: 2,
            },
        ],
    };
}

/// Every object is a quad: CCW corners, two triangles; only the textured quads use uv.
pub(crate) const VERTICES: [Vertex; 24] = [
    // Floor: a small ground plane at y = -1.5, 3.2 x 4 meters.
    Vertex {
        position: [-1.6, -1.5, 2.0, 1.0],
        color: FLOOR_COLOR,
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [1.6, -1.5, 2.0, 1.0],
        color: FLOOR_COLOR,
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [1.6, -1.5, -2.0, 1.0],
        color: FLOOR_COLOR,
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [-1.6, -1.5, -2.0, 1.0],
        color: FLOOR_COLOR,
        uv: [0.0, 0.0],
    },
    // World axes as thin strips in the z = 0 plane: +X to the right...
    Vertex {
        position: [0.0, -0.02, 0.0, 1.0],
        color: AXIS_X_COLOR,
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [2.2, -0.02, 0.0, 1.0],
        color: AXIS_X_COLOR,
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [2.2, 0.02, 0.0, 1.0],
        color: AXIS_X_COLOR,
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [0.0, 0.02, 0.0, 1.0],
        color: AXIS_X_COLOR,
        uv: [0.0, 0.0],
    },
    // ...+Y up...
    Vertex {
        position: [-0.02, 0.0, 0.0, 1.0],
        color: AXIS_Y_COLOR,
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [0.02, 0.0, 0.0, 1.0],
        color: AXIS_Y_COLOR,
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [0.02, 2.2, 0.0, 1.0],
        color: AXIS_Y_COLOR,
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [-0.02, 2.2, 0.0, 1.0],
        color: AXIS_Y_COLOR,
        uv: [0.0, 0.0],
    },
    // ...and +Z toward the camera, lifted by its half thickness so it is not seen edge-on.
    Vertex {
        position: [-0.02, 0.05, 0.0, 1.0],
        color: AXIS_Z_COLOR,
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [0.02, 0.05, 0.0, 1.0],
        color: AXIS_Z_COLOR,
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [0.02, 0.05, 2.0, 1.0],
        color: AXIS_Z_COLOR,
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [-0.02, 0.05, 2.0, 1.0],
        color: AXIS_Z_COLOR,
        uv: [0.0, 0.0],
    },
    // Far quad: 0.8 x 0.8 m textured square in the z = 1 plane, 4 m from the eye.
    Vertex {
        position: [-0.4, -1.0, 1.0, 1.0],
        color: [0.0; 4],
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [0.4, -1.0, 1.0, 1.0],
        color: [0.0; 4],
        uv: [1.0, 0.0],
    },
    Vertex {
        position: [0.4, -0.2, 1.0, 1.0],
        color: [0.0; 4],
        uv: [1.0, 1.0],
    },
    Vertex {
        position: [-0.4, -0.2, 1.0, 1.0],
        color: [0.0; 4],
        uv: [0.0, 1.0],
    },
    // Near quad: the same size in the z = 3 plane, 2 m from the eye.
    Vertex {
        position: [-0.4, 0.2, 3.0, 1.0],
        color: [0.0; 4],
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [0.4, 0.2, 3.0, 1.0],
        color: [0.0; 4],
        uv: [1.0, 0.0],
    },
    Vertex {
        position: [0.4, 1.0, 3.0, 1.0],
        color: [0.0; 4],
        uv: [1.0, 1.0],
    },
    Vertex {
        position: [-0.4, 1.0, 3.0, 1.0],
        color: [0.0; 4],
        uv: [0.0, 1.0],
    },
];

pub(crate) const INDICES: [u16; 36] = [
    0, 1, 2, 0, 2, 3, // floor
    4, 5, 6, 4, 6, 7, // +X axis
    8, 9, 10, 8, 10, 11, // +Y axis
    12, 13, 14, 12, 14, 15, // +Z axis
    16, 17, 18, 16, 18, 19, // far quad
    20, 21, 22, 20, 22, 23, // near quad
];

