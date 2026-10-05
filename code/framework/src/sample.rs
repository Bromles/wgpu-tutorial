use std::error::Error;

use winit::{
    event::{DeviceEvent, WindowEvent},
    window::Window,
};

use crate::gpu::Gpu;
use wgpu::CommandEncoder;
use wgpu::TextureView;

/// Chapter-local drawing code: the framework calls it, never the other way round.
pub trait Sample: 'static {
    /// Creates resources; called again after every surface loss, with the new device.
    fn init(gpu: &Gpu) -> Result<Self, Box<dyn Error>>
    where
        Self: Sized;

    /// Records one frame into `encoder`; the framework submits, presents, owns the frame.
    fn draw(&mut self, gpu: &Gpu, encoder: &mut CommandEncoder, view: &TextureView);

    /// Called right after `init`, before the first `draw`, and on every resize;
    /// zero sizes are filtered out by the framework. Optional.
    fn resize(&mut self, _width: u32, _height: u32) {}

    /// Raw window-event forwarding while a context exists; during the ~100 ms
    /// recovery after a surface loss (and while suspended) the sample is
    /// absent and its events are dropped. Optional.
    fn window_event(&mut self, _window: &Window, _event: &WindowEvent) {}

    /// Raw device-event forwarding (e.g. relative mouse motion) while a
    /// context exists. Optional.
    fn device_event(&mut self, _event: &DeviceEvent) {}
}
