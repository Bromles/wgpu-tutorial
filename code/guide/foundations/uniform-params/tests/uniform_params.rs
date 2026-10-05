use wgpu::TextureFormat;
use verify::{ComparisonType, compare_reference, gpu_context, reference_path, render_and_readback};
use framework::{Gpu, Sample};
use uniform_params::params::Params;
use uniform_params::sample::UniformParams;

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

/// Snapshot 08b: default params reproduce the 07b frame; then gain = 0.5
/// must scale every pixel in linear light, i.e. byte' = encode(0.5 * decode(byte)).
#[test]
fn uniform_params_gain_scales_linear_light() {
    let Some(ctx) = gpu_context() else {
        println!("skip: no suitable GPU adapter");
        return;
    };
    let (width, height) = (768, 576);
    let gpu = Gpu {
        device: ctx.device.clone(),
        queue: ctx.queue.clone(),
        format: TextureFormat::Rgba8UnormSrgb,
    };
    let mut sample = UniformParams::init(&gpu).expect("init sample");
    let neutral = render_and_readback(&ctx, width, height, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    let path = reference_path("indexed-geometry.png");
    compare_reference(
        &path,
        &ctx.adapter_info,
        width,
        height,
        &neutral,
        &[
            ComparisonType::Mean(0.0),
            ComparisonType::Percentile {
                percentile: 0.99,
                threshold: 0.0,
            },
        ],
    );

    sample.set_params(
        &gpu,
        &Params {
            gain: 0.5,
            ..Params::default()
        },
    );
    let halved = render_and_readback(&ctx, width, height, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    // Gain multiplies only the geometry; the clear stays 0.5. Quad: x 96..672, y 72..504.
    let in_quad = |index: usize| {
        let px = (index / 4) % width as usize;
        let py = (index / 4) / width as usize;
        (96..672).contains(&px) && (72..504).contains(&py)
    };
    for (index, (before, after)) in neutral.iter().zip(&halved).enumerate() {
        if index % 4 == 3 || !in_quad(index) {
            continue;
        }
        let expected = (srgb_encode(srgb_decode(f32::from(*before) / 255.0) * 0.5) * 255.0).round();
        let delta = (expected - f32::from(*after)).abs();
        assert!(
            delta <= 1.0,
            "pixel byte {index}: linear halving of {before} expected {expected}, got {after}"
        );
    }
}
