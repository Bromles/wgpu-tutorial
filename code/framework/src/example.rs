use std::time::Duration;

use wgpu::{CommandEncoder, TextureView};
use winit::dpi::PhysicalSize;

use crate::GpuContext;
use crate::Input;

pub trait Example: 'static {
    fn init(ctx: &GpuContext) -> Self;
    fn resize(&mut self, _ctx: &GpuContext, _new_size: PhysicalSize<u32>) {}
    fn update(&mut self, _ctx: &GpuContext, _dt: Duration, _input: &Input) {}
    fn render(&mut self, ctx: &GpuContext, view: &TextureView, encoder: &mut CommandEncoder);
}
