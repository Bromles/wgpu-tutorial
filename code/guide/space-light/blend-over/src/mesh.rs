//! Chapter mesh: vertices, builders, index ranges.

use crate::params::{BLACK, BLUE, GREEN, HALF_X, HALF_Y, RED, Z};
use bytemuck::Pod;
use bytemuck::Zeroable;
use std::ops::Range;
use wgpu::{VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct Vertex {
    /// Position with w = 1: a point of the z = Z plane.
    pub(crate) position: [f32; 4],
    /// Linear RGBA; opaque quads carry a = 1, sources a = 0.5.
    pub(crate) color: [f32; 4],
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

/// One axis-aligned rectangle of the design plane.
pub(crate) fn quad(x0: f32, y0: f32, x1: f32, y1: f32, color: [f32; 4]) -> [Vertex; 4] {
    [
        Vertex {
            position: [x0, y0, Z, 1.0],
            color,
        },
        Vertex {
            position: [x1, y0, Z, 1.0],
            color,
        },
        Vertex {
            position: [x0, y1, Z, 1.0],
            color,
        },
        Vertex {
            position: [x1, y1, Z, 1.0],
            color,
        },
    ]
}

/// Backgrounds first, then sources; preset A left, preset B right.
pub(crate) fn vertices() -> [Vertex; 20] {
    let quads = [
        quad(-HALF_X, -HALF_Y, 0.0, HALF_Y, BLUE),
        quad(0.0, -HALF_Y, HALF_X, HALF_Y, BLACK),
        quad(-1.8, -0.9, -0.6, 0.9, RED),
        quad(0.3, -0.9, 1.5, 0.9, RED),
        quad(0.9, -0.9, 2.1, 0.9, GREEN),
    ];
    let mut vertices = [Vertex {
        position: [0.0; 4],
        color: [0.0; 4],
    }; 20];
    for (quad, chunk) in quads.iter().zip(vertices.as_chunks_mut::<4>().0.iter_mut()) {
        chunk.copy_from_slice(quad);
    }
    vertices
}

/// Six indices per quad, each block addressing its own four vertices.
pub(crate) fn indices() -> [u16; 30] {
    let mut indices = [0u16; 30];
    for (q, block) in indices.as_chunks_mut::<6>().0.iter_mut().enumerate() {
        let base = (q * 4) as u16;
        block.copy_from_slice(&[base, base + 1, base + 2, base + 2, base + 1, base + 3]);
    }
    indices
}

/// Index ranges of the two passes: backgrounds (opaque) and sources.
pub(crate) const BACKGROUNDS: Range<u32> = 0..12;
pub(crate) const SOURCES: Range<u32> = 12..30;
