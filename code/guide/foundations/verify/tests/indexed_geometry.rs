use foundations_verify::{ComparisonType, gpu_context, reference_path, render_and_readback};
use indexed_geometry::sample::IndexedGeometry;
use shell::{Gpu, Sample};

fn pixel(bytes: &[u8], width: u32, px: u32, py: u32) -> [u8; 4] {
    let offset = ((py * width + px) * 4) as usize;
    bytes[offset..offset + 4].try_into().unwrap()
}

/// Snapshot 07b: indexed rectangle; corner regions are dominated by their
/// corner colors, then the frame goes through the reference comparison.
#[test]
fn indexed_geometry_matches_reference() {
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
    let mut sample = IndexedGeometry::init(&gpu).expect("init sample");
    let bytes = render_and_readback(&ctx, width, height, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });

    // Quad spans x 96..672, y 72..504; corner probes stay corner-dominated.
    for (px, py, dominant) in [(110, 86, 0), (658, 86, 1), (110, 490, 2)] {
        let p = pixel(&bytes, width, px, py);
        let max_channel = (0..3).max_by_key(|&c| p[c]).expect("three channels");
        assert_eq!(
            max_channel, dominant,
            "pixel ({px},{py}) expected channel {dominant} to dominate, got {p:?}"
        );
    }
    // The bottom-right corner is white: all channels are high there.
    let white = pixel(&bytes, width, 658, 490);
    assert!(
        white[..3].iter().all(|&c| c > 200),
        "bottom-right corner expected near-white, got {white:?}"
    );

    let path = reference_path("indexed-geometry.png");
    foundations_verify::compare_reference(
        &path,
        &ctx.adapter_info,
        width,
        height,
        &bytes,
        &[
            ComparisonType::Mean(0.02),
            ComparisonType::Percentile {
                percentile: 0.99,
                threshold: 0.35,
            },
        ],
    );
}
