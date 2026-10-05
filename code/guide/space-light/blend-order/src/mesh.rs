//! Chapter mesh: vertices, builders, index ranges.

use crate::params::{
    BLACK, CUT_ABOVE, CUT_BELOW, GREEN, HALF_X, HALF_Y, RED, Z_BACKGROUND, Z_SOURCES,
};
use bytemuck::Pod;
use bytemuck::Zeroable;
use std::ops::Range;
use wgpu::{VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct Vertex {
    /// Position with w = 1: a point of the design plane at its layer's z.
    pub(crate) position: [f32; 4],
    /// Linear RGBA; the cutout halves carry 0.49 / 0.51 instead of 0.5.
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

/// One axis-aligned rectangle from its bounds, depth and constant color.
pub(crate) fn quad(x0: f32, y0: f32, x1: f32, y1: f32, z: f32, color: [f32; 4]) -> [Vertex; 4] {
    [
        Vertex {
            position: [x0, y0, z, 1.0],
            color,
        },
        Vertex {
            position: [x1, y0, z, 1.0],
            color,
        },
        Vertex {
            position: [x0, y1, z, 1.0],
            color,
        },
        Vertex {
            position: [x1, y1, z, 1.0],
            color,
        },
    ]
}

/// Preset B of 27a: the whole frame is the opaque black background.
pub(crate) fn background_vertices() -> [Vertex; 4] {
    quad(-HALF_X, -HALF_Y, HALF_X, HALF_Y, Z_BACKGROUND, BLACK)
}

pub(crate) fn blend_vertices() -> [Vertex; 8] {
    let red = quad(-1.5, -0.9, -0.3, 0.9, Z_SOURCES, RED);
    let green = quad(-0.9, -0.9, 0.3, 0.9, Z_SOURCES, GREEN);
    let mut vertices = [Vertex {
        position: [0.0; 4],
        color: [0.0; 4],
    }; 8];
    for (part, chunk) in [red, green]
        .iter()
        .zip(vertices.as_chunks_mut::<4>().0.iter_mut())
    {
        chunk.copy_from_slice(part);
    }
    vertices
}

/// Each source splits into halves with alpha just below/above the threshold.
pub(crate) fn cutout_vertices() -> [Vertex; 16] {
    let quads = [
        quad(
            -1.5,
            0.0,
            -0.3,
            0.9,
            Z_SOURCES,
            [RED[0], RED[1], RED[2], CUT_BELOW],
        ),
        quad(
            -1.5,
            -0.9,
            -0.3,
            0.0,
            Z_SOURCES,
            [RED[0], RED[1], RED[2], CUT_ABOVE],
        ),
        quad(
            -0.9,
            0.0,
            0.3,
            0.9,
            Z_SOURCES,
            [GREEN[0], GREEN[1], GREEN[2], CUT_BELOW],
        ),
        quad(
            -0.9,
            -0.9,
            0.3,
            0.0,
            Z_SOURCES,
            [GREEN[0], GREEN[1], GREEN[2], CUT_ABOVE],
        ),
    ];
    let mut vertices = [Vertex {
        position: [0.0; 4],
        color: [0.0; 4],
    }; 16];
    for (part, chunk) in quads.iter().zip(vertices.as_chunks_mut::<4>().0.iter_mut()) {
        chunk.copy_from_slice(part);
    }
    vertices
}

/// Six indices per quad; bases restart from zero per vertex buffer.
pub(crate) fn indices() -> [u16; 42] {
    const BASES: [u16; 7] = [0, 0, 4, 0, 4, 8, 12];
    let mut indices = [0u16; 42];
    for (q, block) in indices.as_chunks_mut::<6>().0.iter_mut().enumerate() {
        let base = BASES[q];
        block.copy_from_slice(&[base, base + 1, base + 2, base + 2, base + 1, base + 3]);
    }
    indices
}

/// Index blocks of the three vertex buffers, in buffer order.
pub(crate) const BACKGROUND: Range<u32> = 0..6;
pub(crate) const BLEND_RED: Range<u32> = 6..12;
pub(crate) const BLEND_GREEN: Range<u32> = 12..18;
pub(crate) const CUT_RED_UPPER: Range<u32> = 18..24;
pub(crate) const CUT_RED_LOWER: Range<u32> = 24..30;
pub(crate) const CUT_GREEN_UPPER: Range<u32> = 30..36;
pub(crate) const CUT_GREEN_LOWER: Range<u32> = 36..42;
