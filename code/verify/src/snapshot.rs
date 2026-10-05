//! Shared kit for the per-chapter `snapshot` examples: each chapter renders
//! its own article images into `docs/public/results` through these helpers.

use std::path::{Path, PathBuf};

use framework::{Gpu, Sample};
use wgpu::TextureFormat;

use crate::context::GpuContext;
use crate::png_io::write_png;
use crate::readback::render_and_readback;
use std::fs::create_dir_all;


/// Article frame size shared by every chapter snapshot.
use std::collections::HashSet;
pub const W: u32 = 768;
pub const H: u32 = 576;

pub fn results_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/public/results")
}

pub fn gpu_of(ctx: &GpuContext) -> Gpu {
    Gpu {
        device: ctx.device.clone(),
        queue: ctx.queue.clone(),
        format: TextureFormat::Rgba8UnormSrgb,
    }
}

pub fn render<S: Sample>(ctx: &GpuContext,
sample: &mut S) -> Vec<u8> {
    render_and_readback(ctx, W, H, |encoder, view| {
        sample.draw(&gpu_of(ctx), encoder, view)
    })
}

/// Sanity guard: an article image must not be a flat fill.
fn check(name: &str, width: u32, height: u32, bytes: &[u8]) { let mut min = [255u8; 3]; let mut max = [0u8; 3]; let mut unique = HashSet::new(); for px in bytes.as_chunks::<4>().0 { for c in 0..3 { min[c] = min[c].min(px[c]); max[c] = max[c].max(px[c]);
        }
        unique.insert([px[0], px[1], px[2]]);
    }
    let spread: u32 = (0..3).map(|c| u32::from(max[c] - min[c])).sum();
    assert!(
        spread > 60,
        "{name}: degenerate image (spread {spread}, {} colors)",
        unique.len()
    );
    println!("{name}: {}x{} colors={}", width, height, unique.len());
}

pub fn save(name: &str,
width: u32,
height: u32,
bytes: &[u8]) {
    create_dir_all(results_dir()).expect("create results directory");
    let path = results_dir().join(name);
    write_png(&path, width, height, bytes);
    check(name, width, height, bytes);
}

pub fn side_by_side(a: &[u8],
b: &[u8],
width: u32,
height: u32) -> Vec<u8> {
    let gap = 8;
    let full = width * 2 + gap;
    let mut out = vec![0u8; (full * height * 4) as usize];
    for y in 0..height {
        let src = (y * width * 4) as usize;
        let dst = (y * full * 4) as usize;
        out[dst..dst + (width * 4) as usize].copy_from_slice(&a[src..src + (width * 4) as usize]);
        let dst2 = dst + ((width + gap) * 4) as usize;
        out[dst2..dst2 + (width * 4) as usize].copy_from_slice(&b[src..src + (width * 4) as usize]);
    }
    out
}

pub fn crop_zoom(
    bytes: &[u8],
    x0: u32,
    y0: u32,
    cw: u32,
    ch: u32,
    zoom: u32,
) -> (Vec<u8>, u32, u32) {
    let mut out = vec![0u8; ((cw * zoom) * (ch * zoom) * 4) as usize];
    for dy in 0..ch * zoom {
        for dx in 0..cw * zoom {
            let sx = x0 + dx / zoom;
            let sy = y0 + dy / zoom;
            let src = ((sy * W + sx) * 4) as usize;
            let dst = ((dy * cw * zoom + dx) * 4) as usize;
            out[dst..dst + 4].copy_from_slice(&bytes[src..src + 4]);
        }
    }
    (out, cw * zoom, ch * zoom)
}
