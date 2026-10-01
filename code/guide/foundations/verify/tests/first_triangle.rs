use first_triangle::Sample;
use foundations_verify::{ComparisonType, gpu_context, reference_path, render_and_readback};

/// Snapshot 04: white triangle on the gray background, 768x576 offscreen.
#[test]
fn first_triangle_matches_reference() {
    let Some(ctx) = gpu_context() else {
        println!("skip: no suitable GPU adapter");
        return;
    };
    let (width, height) = (768, 576);
    let sample = Sample::new(&ctx.device, wgpu::TextureFormat::Rgba8UnormSrgb);
    let bytes = render_and_readback(&ctx, width, height, |encoder, view| {
        sample.draw(encoder, view);
    });

    // Probes independent of the reference: (384, 288) is NDC (0, 0), inside.
    let inside = pixel(&bytes, width, 384, 288);
    assert_eq!(&inside[..3], &[255, 255, 255], "triangle interior is white");
    assert_eq!(inside[3], 255, "triangle interior is opaque");
    // Pixel (5, 5) is far outside: the sRGB encoding of linear 0.5 gray.
    let outside = pixel(&bytes, width, 5, 5);
    for channel in &outside[..3] {
        assert!(
            (188 - i32::from(*channel)).abs() <= 1,
            "background byte {channel} is not the expected sRGB code of 0.5"
        );
    }

    let path = reference_path("first-triangle.png");
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

fn pixel(bytes: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let offset = ((y * width + x) * 4) as usize;
    bytes[offset..offset + 4].try_into().unwrap()
}
