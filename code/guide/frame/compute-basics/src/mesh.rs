//! Strip mesh and the shared frame parameter.

use wgpu::{VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode};

use crate::sample::CELLS;
use bytemuck::Pod;
use bytemuck::Zeroable;

/// Frame parameter shared by both passes; a lone u32 is a valid uniform.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct Params {
    pub(crate) count: u32,
}

/// One corner of one indicator cell.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct Vertex {
    /// Packed tight: read by explicit offsets, not WGSL struct rules.
    pub(crate) corner: [f32; 2],
    pub(crate) cell: u32,
}

impl Vertex {
    pub(crate) const LAYOUT: VertexBufferLayout<'static> = VertexBufferLayout {
        array_stride: 12,
        step_mode: VertexStepMode::Vertex,
        attributes: &[
            VertexAttribute {
                format: VertexFormat::Float32x2,
                offset: 0,
                shader_location: 0,
            },
            VertexAttribute {
                format: VertexFormat::Uint32,
                offset: 8,
                shader_location: 1,
            },
        ],
    };
}

/// 257 quads; the vertex stage scales each by its compute-written value.
pub(crate) fn build_strip_mesh() -> (Vec<Vertex>, Vec<u16>) {
    let mut vertices = Vec::with_capacity(CELLS as usize * 4);
    let mut indices = Vec::with_capacity(CELLS as usize * 6);
    for cell in 0..CELLS as u16 {
        let base = cell * 4;
        for corner in [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]] {
            vertices.push(Vertex {
                corner,
                cell: u32::from(cell),
            });
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base + 1, base + 3, base + 2]);
    }
    (vertices, indices)
}
