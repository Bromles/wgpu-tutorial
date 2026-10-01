use foundations_verify::{
    ComparisonType, compare_reference, gpu_context, reference_path, render_and_readback,
};

const WIDTH: u32 = 768;
const HEIGHT: u32 = 576;

/// Snapshots 03a/03b client area: a uniform sRGB-gray clear. Presented
/// pixels can't be captured, so the same offscreen clear is the reference.
#[test]
fn clear_frame_matches_the_gray_reference() {
    let Some(ctx) = gpu_context() else {
        println!("skip: no suitable GPU adapter");
        return;
    };

    let bytes = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Verify clear pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.5,
                        g: 0.5,
                        b: 0.5,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        drop(pass);
    });

    for name in ["first-frame.png", "surface-lifecycle.png"] {
        compare_reference(
            &reference_path(name),
            &ctx.adapter_info,
            WIDTH,
            HEIGHT,
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
