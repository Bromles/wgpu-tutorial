use wgpu::{CommandEncoder, TextureView};
use winit::dpi::PhysicalSize;

use crate::GpuContext;

pub trait Example: 'static {
    fn init(ctx: &GpuContext) -> Self;
    fn resize(&mut self, ctx: &GpuContext, new_size: PhysicalSize<u32>);
    fn render(&mut self, ctx: &GpuContext, view: &TextureView, encoder: &mut CommandEncoder);
}
