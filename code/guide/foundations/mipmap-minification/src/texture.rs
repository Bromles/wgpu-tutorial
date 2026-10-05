use framework::Gpu;
use wgpu::{
    Extent3d, TexelCopyBufferLayout, TexelCopyTextureInfo, Texture, TextureAspect,
    TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView,
    TextureViewDescriptor,
};

/// Side of the square checkerboard texture: 8x8 texels, mip chain 8, 4, 2, 1.
use wgpu::Origin3d;
pub const SIZE: u32 = 8;
// The chain 8, 4, 2, 1 has exactly ilog2(SIZE) + 1 levels.
pub const MIP_LEVELS: u32 = SIZE.ilog2() + 1;

fn srgb_encode(linear: f32) -> f32 {
    if linear <= 0.003_130_8 {
        12.92 * linear
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    }
}

fn srgb_decode(code: f32) -> f32 {
    if code <= 0.040_45 {
        code / 12.92
    } else {
        ((code + 0.055) / 1.055).powf(2.4)
    }
}

///  The level 0 checkerboard: one black or white texel per cell.
fn checkerboard() -> Vec<u8> { let mut texels = vec![0u8; (SIZE * SIZE * 4) as usize]; for y in 0..SIZE { for x in 0..SIZE { let value = if (x + y) % 2 == 0 { 255 } else { 0 }; let offset = ((y * SIZE + x) * 4) as usize; texels[offset..offset + 4].copy_from_slice(&[value, value, value, 255]);
        }
    }
    texels
}

/// Each level averages 2x2 blocks of the previous one in linear light, then re-encodes.
fn mip_chain() -> Vec<Vec<u8>> { let mut levels = vec![checkerboard()]; while levels.last().unwrap().len() > 4 { let source = levels.last().unwrap(); let source_side = ((source.len() / 4) as f64).sqrt() as u32; let target_side = source_side / 2; let mut target = vec![0u8; (target_side * target_side * 4) as usize]; for y in 0..target_side { for x in 0..target_side { let mut linear_sum = 0.0; for (dy, dx) in [(0u32, 0u32), (0, 1), (1, 0), (1, 1)] { let sx = x * 2 + dx; let sy = y * 2 + dy; let offset = ((sy * source_side + sx) * 4) as usize; linear_sum += srgb_decode(f32::from(source[offset]) / 255.0);
                }
                let encoded = (srgb_encode(linear_sum / 4.0) * 255.0).round() as u8;
                let offset = ((y * target_side + x) * 4) as usize;
                target[offset..offset + 4].copy_from_slice(&[encoded, encoded, encoded, 255]);
            }
        }
        levels.push(target);
    }
    levels
}

pub fn create(gpu: &Gpu) -> (Texture, TextureView) {
    let texture = gpu.device.create_texture(&TextureDescriptor {
        label: Some("Checkerboard mip chain"),
        size: Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        mip_level_count: MIP_LEVELS,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba8UnormSrgb,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (level, texels) in mip_chain().into_iter().enumerate() {
        let side = (texels.len() as f64 / 4.0).sqrt() as u32;
        // Queue upload: tight rows, no 256-alignment needed.
        gpu.queue.write_texture(
            TexelCopyTextureInfo {
                texture: &texture,
                mip_level: level as u32,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            &texels,
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: if side == 1 { None } else { Some(4 * side) },
                rows_per_image: None,
            },
            Extent3d {
                width: side,
                height: side,
                depth_or_array_layers: 1,
            },
        );
    }
    let view = texture.create_view(&TextureViewDescriptor {
        label: Some("Checkerboard mip view"),
        ..TextureViewDescriptor::default()
    });
    (texture, view)
}
