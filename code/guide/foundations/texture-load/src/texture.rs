use framework::Gpu;
use wgpu::{
    Extent3d, TexelCopyBufferLayout, TexelCopyTextureInfo, Texture, TextureAspect,
    TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView,
    TextureViewDescriptor,
};

/// Chapter 01 image: 2x2 RGBA8 sRGB codes, row-major from the top-left.
pub const WIDTH: u32 = 2;
pub const HEIGHT: u32 = 2;
pub const TEXELS: [u8; WIDTH as usize * HEIGHT as usize * 4] = [
    255, 0, 0, 255, // (0, 0): red
    0, 255, 0, 255, // (1, 0): green
    0, 0, 255, 255, // (0, 1): blue
    255, 255, 255, 255, // (1, 1): white
];

pub fn create(gpu: &Gpu) -> (Texture, TextureView) {
    let texture = gpu.device.create_texture(&TextureDescriptor {
        label: Some("2x2 sRGB texture"),
        size: Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba8UnormSrgb,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        view_formats: &[],
    });
    // Queue upload: tight rows, no 256-alignment (that rule is for encoder copies).
    gpu.queue.write_texture(
        TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: TextureAspect::All,
        },
        &TEXELS,
        TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * WIDTH),
            rows_per_image: None,
        },
        Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
    );
    let view = texture.create_view(&TextureViewDescriptor {
        label: Some("2x2 texture view"),
        ..TextureViewDescriptor::default()
    });
    (texture, view)
}
