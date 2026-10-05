//! Intermediate halving targets with their views and bind groups.

use framework::Gpu;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, Extent3d, Texture,
    TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView,
    TextureViewDescriptor,
};
use wgpu::BindingResource;

/// A sized RGBA8Unorm image with its view and bind group.
pub(crate) struct Intermediate { pub(crate) _texture: Texture, pub(crate) view: TextureView, pub(crate) bind_group: BindGroup,
}

pub(crate) fn create_intermediate(
    gpu: &Gpu,
    layout: &BindGroupLayout,
    label: &str,
    width: u32,
    height: u32,
    usage: TextureUsages,
) -> Intermediate {
    let texture = gpu.device.create_texture(&TextureDescriptor {
        label: Some(label),
        size: Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba8Unorm,
        usage,
        view_formats: &[],
    });
    let view = texture.create_view(&TextureViewDescriptor {
        label: Some(label),
        ..TextureViewDescriptor::default()
    });
    let bind_group = gpu.device.create_bind_group(&BindGroupDescriptor {
        label: Some(label),
        layout,
        entries: &[BindGroupEntry {
            binding: 0,
            resource: BindingResource::TextureView(&view),
        }],
    });
    Intermediate {
        _texture: texture,
        view,
        bind_group,
    }
}

