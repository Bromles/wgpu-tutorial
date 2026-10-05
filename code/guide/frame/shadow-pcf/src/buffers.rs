//! Uniform buffers and the main-pass bind group.

use framework::Gpu;
use wgpu::{BindingResource, 
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, Buffer, BufferBinding,
    BufferDescriptor, BufferUsages, TextureView,
};

/// Creates a uniform buffer; 64 bytes per mat4x4 or the params.
pub(crate) fn create_uniform(gpu: &Gpu, label: &str, size: u64) -> Buffer { gpu.device.create_buffer(&BufferDescriptor { label: Some(label), size, usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST, mapped_at_creation: false,
    })
}

/// Shared inputs of every main-pass bind group.
pub(crate) struct MainBuffers<'a> { pub(crate) layout: &'a BindGroupLayout, pub(crate) camera: &'a Buffer, pub(crate) light: &'a Buffer, pub(crate) params: &'a Buffer,
}

/// Five entries; only the model buffer differs between floor and cube.
pub(crate) fn create_main_bind_group( gpu: &Gpu, label: &str, model: &Buffer, shared: &MainBuffers, shadow: &TextureView,
) -> BindGroup {
    gpu.device.create_bind_group(&BindGroupDescriptor {
        label: Some(label),
        layout: shared.layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: model,
                    offset: 0,
                    size: None,
                }),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: shared.camera,
                    offset: 0,
                    size: None,
                }),
            },
            BindGroupEntry {
                binding: 2,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: shared.light,
                    offset: 0,
                    size: None,
                }),
            },
            BindGroupEntry {
                binding: 3,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: shared.params,
                    offset: 0,
                    size: None,
                }),
            },
            BindGroupEntry {
                binding: 4,
                resource: BindingResource::TextureView(shadow),
            },
        ],
    })
}
