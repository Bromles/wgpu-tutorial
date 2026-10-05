//! Offscreen render target and GPU readback.

use std::sync::mpsc::channel;

use wgpu::{
    Buffer, BufferDescriptor, BufferUsages, CommandEncoder, CommandEncoderDescriptor, Extent3d,
    MapMode, Origin3d, PollType, TexelCopyBufferInfo, TexelCopyBufferLayout, TexelCopyTextureInfo,
    Texture, TextureAspect, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    TextureView, TextureViewDescriptor,
};

use crate::context::GpuContext;

/// Offscreen render + readback; returns tightly packed sRGB bytes. Any
/// width works: the 256-byte row rule is absorbed by the copy and compacted.
pub fn render_and_readback(
    ctx: &GpuContext,
    width: u32,
    height: u32,
    draw: impl FnOnce(&mut CommandEncoder, &TextureView),
) -> Vec<u8> {
    const ROW_ALIGNMENT: u32 = 256;
    let tight_row = width * 4;
    let padded_row = tight_row.div_ceil(ROW_ALIGNMENT) * ROW_ALIGNMENT;
    let texture = ctx.device.create_texture(&TextureDescriptor {
        label: Some("Verify target"),
        size: Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba8UnormSrgb,
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&TextureViewDescriptor::default());
    let buffer = ctx.device.create_buffer(&BufferDescriptor {
        label: Some("Verify readback"),
        size: u64::from(padded_row) * u64::from(height),
        usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = ctx
        .device
        .create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Verify encoder"),
        });
    draw(&mut encoder, &view);
    copy_texture_to_buffer(&mut encoder, &texture, &buffer, width, height, padded_row);
    ctx.queue.submit([encoder.finish()]);

    let slice = buffer.slice(..);
    let (sender, receiver) = channel();
    slice.map_async(MapMode::Read, move |result| {
        sender.send(result).expect("receiver alive");
    });
    ctx.device
        .poll(PollType::wait_indefinitely())
        .expect("device is not lost");
    receiver
        .recv()
        .expect("map callback ran")
        .expect("map succeeds");
    let padded = slice
        .get_mapped_range()
        .expect("buffer is mapped after poll")
        .to_vec();
    buffer.unmap();
    // Compact: drop the inter-row padding the copy had to insert.
    if padded_row == tight_row {
        return padded;
    }
    let mut bytes = Vec::with_capacity((tight_row * height) as usize);
    for row in 0..height as usize {
        let start = row * padded_row as usize;
        bytes.extend_from_slice(&padded[start..start + tight_row as usize]);
    }
    bytes
}

fn copy_texture_to_buffer(
    encoder: &mut CommandEncoder,
    texture: &Texture,
    buffer: &Buffer,
    width: u32,
    height: u32,
    padded_row: u32,
) {
    encoder.copy_texture_to_buffer(
        TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        },
        TexelCopyBufferInfo {
            buffer,
            layout: TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_row),
                rows_per_image: None,
            },
        },
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
}
