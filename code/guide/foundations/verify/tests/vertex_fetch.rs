use foundations_verify::{ComparisonType, gpu_context, reference_path, render_and_readback};
use shell::{Gpu, Sample};
use vertex_fetch::sample::VertexFetch;

/// Snapshot 07a: the same triangle fed from a vertex buffer; against the
/// chapter 06 reference - moving data must not change a byte.
#[test]
fn vertex_fetch_matches_vertex_colors_reference() {
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
    let mut sample = VertexFetch::init(&gpu).expect("init sample");
    let bytes = render_and_readback(&ctx, width, height, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    let path = reference_path("vertex-colors.png");
    foundations_verify::compare_reference(
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
