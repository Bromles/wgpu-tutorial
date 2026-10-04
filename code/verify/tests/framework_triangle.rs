use verify::{ComparisonType, gpu_context, reference_path, render_and_readback};
use framework::{Gpu, Sample};
use framework_triangle::sample::Triangle;

/// Snapshot 05: the same triangle behind the extracted framework. The frame must
/// be pixel-identical to the chapter 04 reference.
#[test]
fn framework_triangle_matches_first_triangle_reference() {
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
    let mut sample = Triangle::init(&gpu).expect("init sample");
    let bytes = render_and_readback(&ctx, width, height, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });

    let path = reference_path("first-triangle.png");
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
