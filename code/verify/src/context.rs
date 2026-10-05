//! Offscreen device/queue context for verification runs.
use pollster::block_on;
use wgpu::{
    AdapterInfo, Backends, Device, DeviceDescriptor, Features, Instance, InstanceDescriptor,
    Limits, Queue, RequestAdapterOptions,
};


/// Adapter, device and queue handed to a chapter sample during verification.
pub struct GpuContext {
    pub device: Device,
    pub queue: Queue,
    pub adapter_info: AdapterInfo,
    /// Adapter's available features; the device only has the requested subset.
    pub adapter_features: Features,
}

/// `None` only when no adapter exists; other failures panic, not skip.
pub fn gpu_context() -> Option<GpuContext> {
    gpu_context_with(DeviceDescriptor {
        label: Some("Foundations verify device"),
        required_features: Features::empty(),
        required_limits: Limits::default(),
        ..Default::default()
    })
}

/// Same, with example-specific device requirements.
pub fn gpu_context_with(device_descriptor: DeviceDescriptor<'static>) -> Option<GpuContext> {
    let instance = Instance::new(InstanceDescriptor {
        backends: Backends::PRIMARY,
        ..InstanceDescriptor::new_without_display_handle()
    });
    let Ok(adapter) = block_on(instance.request_adapter(&RequestAdapterOptions {
        ..Default::default()
    })) else {
        // Genuinely no adapter: the only legitimate skip.
        return None;
    };
    let adapter_features = adapter.features();
    let (device, queue) = block_on(adapter.request_device(&device_descriptor))
        .unwrap_or_else(|error| {
            panic!(
                "adapter found, but the device request failed: {error}. \
             The test's required features/limits exceed what this adapter provides; \
             this is a test requirement problem, not a missing GPU"
            )
        });
    Some(GpuContext {
        device,
        queue,
        adapter_info: adapter.get_info(),
        adapter_features,
    })
}
