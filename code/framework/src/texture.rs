use wgpu::{
    Extent3d, Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    TextureView, TextureViewDescriptor,
};

use crate::GpuContext;

pub fn generate_checkerboard(size: u32, cell_size: u32, light: [u8; 4], dark: [u8; 4]) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            if ((x / cell_size) + (y / cell_size)) % 2 == 0 {
                pixels.extend_from_slice(&light);
            } else {
                pixels.extend_from_slice(&dark);
            }
        }
    }
    pixels
}

pub fn create_depth_texture(ctx: &GpuContext, label: &str) -> (Texture, TextureView) {
    let size = &ctx.surface_config;
    let texture = ctx.device.create_texture(&TextureDescriptor {
        label: Some(label),
        size: Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Depth32Float,
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&TextureViewDescriptor::default());
    (texture, view)
}
