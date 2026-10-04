use verify::{ComparisonType, gpu_context, reference_path, render_and_readback};
use framework::{Gpu, Sample};
use srgb_mixing::sample::SrgbMixing;

fn pixel(bytes: &[u8], width: u32, px: u32, py: u32) -> [u8; 4] {
    let offset = ((py * width + px) * 4) as usize;
    bytes[offset..offset + 4].try_into().unwrap()
}

/// Snapshot 12: the left quad shows encode(mix light) = 188, the right quad
/// shows what the mean of stored codes decodes to, near 128.
#[test]
fn srgb_mixing_probes_and_reference() {
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
    let mut sample = SrgbMixing::init(&gpu).expect("init sample");
    let bytes = render_and_readback(&ctx, width, height, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });

    // Left quad interior: the sRGB code of linear 0.5.
    for px_py in [(192, 288), (100, 200)] {
        let p = pixel(&bytes, width, px_py.0, px_py.1);
        for channel in &p[..3] {
            assert!(
                (188 - i32::from(*channel)).abs() <= 1,
                "left quad at ({},{}) expected 188, got {p:?}",
                px_py.0,
                px_py.1
            );
        }
    }
    // Right quad interior: stored-code mean 127, decoded and re-encoded.
    let expected_right = {
        let wrong = (255u16 / 2) as u8; // 127
        let linear = ((f32::from(wrong) / 255.0 + 0.055) / 1.055).powf(2.4);
        let encoded = if linear <= 0.003_130_8 {
            12.92 * linear
        } else {
            1.055 * linear.powf(1.0 / 2.4) - 0.055
        };
        (encoded * 255.0).round() as u8
    };
    for px_py in [(576, 288), (668, 200)] {
        let p = pixel(&bytes, width, px_py.0, px_py.1);
        for channel in &p[..3] {
            assert!(
                (i32::from(expected_right) - i32::from(*channel)).abs() <= 1,
                "right quad at ({},{}) expected {expected_right}, got {p:?}",
                px_py.0,
                px_py.1
            );
        }
    }

    let path = reference_path("srgb-mixing.png");
    verify::compare_reference(
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
