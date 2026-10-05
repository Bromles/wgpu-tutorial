
use wgpu::Device;
use wgpu::Queue;
use wgpu::TextureFormat;
/// Cloned device/queue handles handed to the sample every frame.
pub struct Gpu {
    pub device: Device,
    pub queue: Queue,
    /// sRGB surface format negotiated in chapter 03.
    pub format: TextureFormat,
}
