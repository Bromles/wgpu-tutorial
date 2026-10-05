use wgpu::{VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode};
use bytemuck::Pod;
use bytemuck::Zeroable;
use std::ops::Range;

/// position has w = 1 (a point), normal w = 0 (a direction).
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 4],
    pub normal: [f32; 4],
}

impl Vertex {
    pub const LAYOUT: VertexBufferLayout<'static> = VertexBufferLayout {
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

/// A mesh is a range of the shared index buffer; cube then floor.
pub const CUBE_MESH: Range<u32> = 0..36;
pub const FLOOR_MESH: Range<u32> = 36..42;

/// Cube corners, CCW from outside so back faces can be culled.
pub fn cube_vertices() -> [Vertex; 24] {
    let face = |positions: [[f32; 3]; 4], normal: [f32; 4]| {
        positions.map(|p| Vertex {
            position: [p[0], p[1], p[2], 1.0],
            normal,
        })
    };
    let mut vertices = [Vertex {
        position: [0.0; 4],
        normal: [0.0; 4],
    }; 24];
    // +Y face (top): screen right is +X, screen up is -Z.
    vertices[0..4].copy_from_slice(&face(
        [
            [-0.5, 0.5, 0.5],
            [0.5, 0.5, 0.5],
            [-0.5, 0.5, -0.5],
            [0.5, 0.5, -0.5],
        ],
        [0.0, 1.0, 0.0, 0.0],
    ));
    vertices[4..8].copy_from_slice(&face(
        [
            [-0.5, -0.5, -0.5],
            [0.5, -0.5, -0.5],
            [-0.5, -0.5, 0.5],
            [0.5, -0.5, 0.5],
        ],
        [0.0, -1.0, 0.0, 0.0],
    ));
    vertices[8..12].copy_from_slice(&face(
        [
            [0.5, -0.5, 0.5],
            [0.5, -0.5, -0.5],
            [0.5, 0.5, 0.5],
            [0.5, 0.5, -0.5],
        ],
        [1.0, 0.0, 0.0, 0.0],
    ));
    vertices[12..16].copy_from_slice(&face(
        [
            [-0.5, -0.5, -0.5],
            [-0.5, -0.5, 0.5],
            [-0.5, 0.5, -0.5],
            [-0.5, 0.5, 0.5],
        ],
        [-1.0, 0.0, 0.0, 0.0],
    ));
    vertices[16..20].copy_from_slice(&face(
        [
            [-0.5, -0.5, 0.5],
            [0.5, -0.5, 0.5],
            [-0.5, 0.5, 0.5],
            [0.5, 0.5, 0.5],
        ],
        [0.0, 0.0, 1.0, 0.0],
    ));
    vertices[20..24].copy_from_slice(&face(
        [
            [0.5, -0.5, -0.5],
            [-0.5, -0.5, -0.5],
            [0.5, 0.5, -0.5],
            [-0.5, 0.5, -0.5],
        ],
        [0.0, 0.0, -1.0, 0.0],
    ));
    vertices
}

/// XZ quad at y = 0; indices are absolute (24..28) after the cube.
pub fn floor_vertices() -> [Vertex; 4] {
    let normal = [0.0, 1.0, 0.0, 0.0];
    [
        Vertex {
            position: [-2.0, 0.0, 2.0, 1.0],
            normal,
        },
        Vertex {
            position: [2.0, 0.0, 2.0, 1.0],
            normal,
        },
        Vertex {
            position: [-2.0, 0.0, -2.0, 1.0],
            normal,
        },
        Vertex {
            position: [2.0, 0.0, -2.0, 1.0],
            normal,
        },
    ]
}

/// One shared pair of buffers: the cube first, the floor after it.
pub fn vertices() -> [Vertex; 28] {
    let mut vertices = [Vertex {
        position: [0.0; 4],
        normal: [0.0; 4],
    }; 28];
    vertices[0..24].copy_from_slice(&cube_vertices());
    vertices[24..28].copy_from_slice(&floor_vertices());
    vertices
}

/// 36 cube indices, then six floor indices over 24..28.
pub fn indices() -> [u16; 42] {
    let mut indices = [0u16; 42];
    for (f, block) in indices
        .as_chunks_mut::<6>()
        .0
        .iter_mut()
        .take(6)
        .enumerate()
    {
        let base = (f * 4) as u16;
        block.copy_from_slice(&[base, base + 1, base + 2, base + 2, base + 1, base + 3]);
    }
    indices[36..42].copy_from_slice(&[24, 25, 26, 26, 25, 27]);
    indices
}

/// The panel vertex record: a flat color needs positions only.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct PanelVertex {
    pub position: [f32; 4],
}

impl PanelVertex {
    pub const LAYOUT: VertexBufferLayout<'static> = VertexBufferLayout {
        array_stride: 16,
        step_mode: VertexStepMode::Vertex,
        attributes: &[VertexAttribute {
            format: VertexFormat::Float32x4,
            offset: 0,
            shader_location: 0,
        }],
    };
}

/// Transparent panel quad; the fragment shader returns a constant.
pub const PANEL_HALF: f32 = 0.55;
pub const PANEL_Y: f32 = 1.5;

pub fn panel_vertices() -> [PanelVertex; 4] {
    [
        PanelVertex {
            position: [-PANEL_HALF, PANEL_Y, PANEL_HALF, 1.0],
        },
        PanelVertex {
            position: [PANEL_HALF, PANEL_Y, PANEL_HALF, 1.0],
        },
        PanelVertex {
            position: [-PANEL_HALF, PANEL_Y, -PANEL_HALF, 1.0],
        },
        PanelVertex {
            position: [PANEL_HALF, PANEL_Y, -PANEL_HALF, 1.0],
        },
    ]
}

pub const PANEL_INDICES: [u16; 6] = [0, 1, 2, 2, 1, 3];
