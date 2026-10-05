//! Chapter mesh: vertices, builders, index ranges.

use bytemuck::Pod;
use bytemuck::Zeroable;
use glam::camera::rh::proj::directx::orthographic;
use glam::{Mat4, Vec3};
use wgpu::{VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode};

/// Static camera pose: eye on the face normal, up hint +Y (sides) or +Z (top/bottom).
pub struct FaceView {
    pub eye: Vec3,
    pub up: Vec3,
}

/// Six views in key order 1..6: +Z, -Z, +X, -X, +Y, -Y.
pub const FACE_VIEWS: [FaceView; 6] = [
    FaceView {
        eye: Vec3::new(0.0, 0.0, 3.0),
        up: Vec3::Y,
    },
    FaceView {
        eye: Vec3::new(0.0, 0.0, -3.0),
        up: Vec3::Y,
    },
    FaceView {
        eye: Vec3::new(3.0, 0.0, 0.0),
        up: Vec3::Y,
    },
    FaceView {
        eye: Vec3::new(-3.0, 0.0, 0.0),
        up: Vec3::Y,
    },
    FaceView {
        eye: Vec3::new(0.0, 3.0, 0.0),
        up: Vec3::Z,
    },
    FaceView {
        eye: Vec3::new(0.0, -3.0, 0.0),
        up: Vec3::Z,
    },
];

/// Fixed ortho volume -2..2 x -1.5..1.5; fits the cube, matches the 4:3 frame.
pub(crate) fn ortho() -> Mat4 {
    orthographic(-2.0, 2.0, -1.5, 1.5, 0.1, 10.0)
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    /// Position with w = 1, padded to four components for 4-byte alignment.
    pub position: [f32; 4],
    /// Texture coordinates of this corner inside this face's copy.
    pub uv: [f32; 2],
}

impl Vertex {
    pub(crate) const LAYOUT: VertexBufferLayout<'static> = VertexBufferLayout {
        array_stride: 24,
        step_mode: VertexStepMode::Vertex,
        attributes: &[
            VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: 0,
                shader_location: 0,
            },
            VertexAttribute {
                format: VertexFormat::Float32x2,
                offset: 16,
                shader_location: 1,
            },
        ],
    };
}

///  The 24 cube corners: four records per face, uv (0,0), (1,0), (0,1), (1,1) as seen from outside.
/// Each corner appears in three records with different UV.
pub const VERTICES: [Vertex; 24] = [
    // +Z face: screen right is +X, screen up is +Y.
    Vertex {
        position: [-0.5, 0.5, 0.5, 1.0],
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [0.5, 0.5, 0.5, 1.0],
        uv: [1.0, 0.0],
    },
    Vertex {
        position: [-0.5, -0.5, 0.5, 1.0],
        uv: [0.0, 1.0],
    },
    Vertex {
        position: [0.5, -0.5, 0.5, 1.0],
        uv: [1.0, 1.0],
    },
    // -Z face: screen right is -X, screen up is +Y.
    Vertex {
        position: [0.5, 0.5, -0.5, 1.0],
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [-0.5, 0.5, -0.5, 1.0],
        uv: [1.0, 0.0],
    },
    Vertex {
        position: [0.5, -0.5, -0.5, 1.0],
        uv: [0.0, 1.0],
    },
    Vertex {
        position: [-0.5, -0.5, -0.5, 1.0],
        uv: [1.0, 1.0],
    },
    // +X face: screen right is -Z, screen up is +Y.
    Vertex {
        position: [0.5, 0.5, 0.5, 1.0],
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [0.5, 0.5, -0.5, 1.0],
        uv: [1.0, 0.0],
    },
    Vertex {
        position: [0.5, -0.5, 0.5, 1.0],
        uv: [0.0, 1.0],
    },
    Vertex {
        position: [0.5, -0.5, -0.5, 1.0],
        uv: [1.0, 1.0],
    },
    // -X face: screen right is +Z, screen up is +Y.
    Vertex {
        position: [-0.5, 0.5, -0.5, 1.0],
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [-0.5, 0.5, 0.5, 1.0],
        uv: [1.0, 0.0],
    },
    Vertex {
        position: [-0.5, -0.5, -0.5, 1.0],
        uv: [0.0, 1.0],
    },
    Vertex {
        position: [-0.5, -0.5, 0.5, 1.0],
        uv: [1.0, 1.0],
    },
    // +Y face (top): screen right is -X, screen up is +Z.
    Vertex {
        position: [0.5, 0.5, 0.5, 1.0],
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [-0.5, 0.5, 0.5, 1.0],
        uv: [1.0, 0.0],
    },
    Vertex {
        position: [0.5, 0.5, -0.5, 1.0],
        uv: [0.0, 1.0],
    },
    Vertex {
        position: [-0.5, 0.5, -0.5, 1.0],
        uv: [1.0, 1.0],
    },
    // -Y face (bottom): screen right is +X, screen up is +Z.
    Vertex {
        position: [-0.5, -0.5, 0.5, 1.0],
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [0.5, -0.5, 0.5, 1.0],
        uv: [1.0, 0.0],
    },
    Vertex {
        position: [-0.5, -0.5, -0.5, 1.0],
        uv: [0.0, 1.0],
    },
    Vertex {
        position: [0.5, -0.5, -0.5, 1.0],
        uv: [1.0, 1.0],
    },
];

// Two triangles per face; the shared diagonal keeps the quad flat in UV.
pub(crate) const INDICES: [u16; 36] = [
    0, 1, 2, 2, 1, 3, // +Z
    4, 5, 6, 6, 5, 7, // -Z
    8, 9, 10, 10, 9, 11, // +X
    12, 13, 14, 14, 13, 15, // -X
    16, 17, 18, 18, 17, 19, // +Y
    20, 21, 22, 22, 21, 23, // -Y
];
