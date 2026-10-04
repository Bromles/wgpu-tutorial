use foundations_verify::{ComparisonType, gpu_context, reference_path, render_and_readback};
use gain_animation::sample::GainAnimation;
use framework::{Gpu, Sample};

fn srgb_encode(x: f32) -> f32 {
    if x <= 0.003_130_8 {
        12.92 * x
    } else {
        1.055 * x.powf(1.0 / 2.4) - 0.055
    }
}

fn srgb_decode(x: f32) -> f32 {
    if x <= 0.040_45 {
        x / 12.92
    } else {
        ((x + 0.055) / 1.055).powf(2.4)
    }
}

fn pixel(bytes: &[u8], width: u32, px: u32, py: u32) -> [u8; 4] {
    let offset = ((py * width + px) * 4) as usize;
    bytes[offset..offset + 4].try_into().unwrap()
}

/// Snapshot 09: at t = 2 s the gain is 0.5 (linear halving of the quad),
/// at t = 0 the quad is black; the reference is the t = 2 frame.
#[test]
fn gain_animation_time_law_holds() {
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
    let mut sample = GainAnimation::init(&gpu).expect("init sample");

    // t = 2: gain = min(0.25 * 2, 1) = 0.5 for both draws.
    sample.set_elapsed(2.0);
    let halved = render_and_readback(&ctx, width, height, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    let reference =
        foundations_verify::read_reference(&reference_path("indexed-geometry.png"), width, height)
            .expect("indexed-geometry reference exists");
    let in_quad = |px: u32, py: u32| (96..672).contains(&px) && (72..504).contains(&py);
    for py in 0..height {
        for px in 0..width {
            if !in_quad(px, py) {
                continue;
            }
            let before = pixel(&reference, width, px, py);
            let after = pixel(&halved, width, px, py);
            for channel in 0..3 {
                let expected = (srgb_encode(srgb_decode(f32::from(before[channel]) / 255.0) * 0.5)
                    * 255.0)
                    .round();
                assert!(
                    (expected - f32::from(after[channel])).abs() <= 1.0,
                    "pixel ({px},{py}) channel {channel}: expected {expected}, got {}",
                    after[channel]
                );
            }
        }
    }

    // t = 0: gain = 0, the whole quad is black, the background is untouched.
    sample.set_elapsed(0.0);
    let black = render_and_readback(&ctx, width, height, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    assert_eq!(&pixel(&black, width, 384, 300)[..3], &[0, 0, 0]);
    for channel in &pixel(&black, width, 10, 10)[..3] {
        assert!((188 - i32::from(*channel)).abs() <= 1);
    }

    // The article image is the t = 2 frame.
    let path = reference_path("gain-animation.png");
    foundations_verify::compare_reference(
        &path,
        &ctx.adapter_info,
        width,
        height,
        &halved,
        &[
            ComparisonType::Mean(0.02),
            ComparisonType::Percentile {
                percentile: 0.99,
                threshold: 0.35,
            },
        ],
    );
}
