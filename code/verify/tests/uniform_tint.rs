use verify::{ComparisonType, gpu_context, reference_path, render_and_readback};
use framework::{Gpu, Sample};
use uniform_tint::sample::UniformTint;

/// Snapshot 08a: the tint is neutral, so the frame must be byte-identical to
/// the chapter 07b reference.
#[test]
fn uniform_tint_matches_indexed_geometry_reference() {
    let Some(ctx) = gpu_context() else {
        println!("skip: no suitable GPU adapter");
        return;
    };
    let (width, height) = (768, 576);
    let gpu = Gpu {
        device: ctx.device.clone(),
        queue: ctx.queue.clone(),
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
    };
    let mut sample = UniformTint::init(&gpu).expect("init sample");
    let bytes = render_and_readback(&ctx, width, height, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    let path = reference_path("indexed-geometry.png");
    verify::compare_reference(
        &path,
        &ctx.adapter_info,
        width,
        height,
        &bytes,
        &[
            ComparisonType::Mean(0.0),
            ComparisonType::Percentile {
                percentile: 0.99,
                threshold: 0.0,
            },
        ],
    );
}
