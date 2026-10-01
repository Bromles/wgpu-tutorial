use foundations_verify::{ComparisonType, gpu_context, reference_path, render_and_readback};
use shell::{Gpu, Sample};
use vertex_pulling::sample::{Mode, VertexPulling};

/// Snapshot 10: at gain = 1 both feeding paths must reproduce the 07b frame
/// byte for byte — fetch and pulling address the same records.
#[test]
fn vertex_pulling_matches_fetch_frame() {
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
    let mut sample = VertexPulling::init(&gpu).expect("init sample");
    sample.set_elapsed(4.0); // gain = min(0.25 * 4, 1) = 1

    let path = reference_path("indexed-geometry.png");
    for mode in [Mode::Fetch, Mode::Pulling] {
        sample.set_mode(mode);
        let bytes = render_and_readback(&ctx, width, height, |encoder, view| {
            sample.draw(&gpu, encoder, view);
        });
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
}
