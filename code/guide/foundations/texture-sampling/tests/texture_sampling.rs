use wgpu::TextureFormat;
use framework::{Gpu, Sample};
use texture_sampling::sample::{Address, Filter, TextureSampling};
use verify::{ComparisonType, compare_reference, gpu_context, reference_path, render_and_readback};

fn pixel(bytes: &[u8], width: u32, px: u32, py: u32) -> [u8; 4] {
    let offset = ((py * width + px) * 4) as usize;
    bytes[offset..offset + 4].try_into().unwrap()
}

/// Snapshot 13b: nearest+clamp reproduces the textureLoad frame byte for
/// byte; bilinear mixes linear colors, so the exact center is code 188.
#[test]
fn texture_sampling_weights_and_modes() {
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
    let mut sample = TextureSampling::init(&gpu).expect("init sample");

    // Nearest + clamp: the same whole texels as textureLoad.
    sample.set_filter(Filter::Nearest);
    sample.set_address(Address::Clamp);
    let nearest = render_and_readback(&ctx, width, height, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    let path = reference_path("texture-load.png");
    compare_reference(
        &path,
        &ctx.adapter_info,
        width,
        height,
        &nearest,
        &[
            ComparisonType::Mean(0.0),
            ComparisonType::Percentile {
                percentile: 0.99,
                threshold: 0.0,
            },
        ],
    );

    // Bilinear + clamp: near-center weights nearly equal -> 0.5 light -> code 188.
    sample.set_filter(Filter::Linear);
    let bilinear = render_and_readback(&ctx, width, height, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    let center = pixel(&bilinear, width, 383, 287);
    for channel in &center[..3] {
        assert!(
            (188 - i32::from(*channel)).abs() <= 1,
            "bilinear center expected code 188, got {center:?}"
        );
    }

    // Bilinear address modes differ INSIDE [0,1]: near the quad corner the
    // filter addresses texel -1 on both axes, and Clamp pins those fetches
    // to the edge texel while Repeat wraps them to the opposite side.
    // Quad spans NDC +-0.75; pixel (96, 72) sits at UV ~(0.0009, 0.0011),
    // giving near-half weights on both axes (~0.499 / ~0.501).
    let corner = (96u32, 72u32);
    let clamp_px = pixel(&bilinear, width, corner.0, corner.1);
    // All four neighbors pinned to texel (0,0) = red: exactly [255, 0, 0].
    let pure_red = [255u8, 0, 0];
    for channel in 0..3 {
        assert!(
            (i32::from(pure_red[channel]) - i32::from(clamp_px[channel])).abs() <= 1,
            "bilinear+clamp corner expected pure red, got {clamp_px:?}"
        );
    }
    sample.set_address(Address::Repeat);
    let repeat = render_and_readback(&ctx, width, height, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    let repeat_px = pixel(&repeat, width, corner.0, corner.1);
    // UV ~(0.0009, 0.0011): both axes address texel -1 with weight ~0.499
    // and texel 0 with ~0.501. Wrapped neighbors: (-1,-1)->(1,1) white,
    // (-1,0)->(1,0) green, (0,-1)->(0,1) blue, (0,0) red - almost equal
    // quarter weights, blue a hair lighter than red/green.
    let outside = 0.499_f32;
    let inside = 1.0 - outside;
    let white = outside * outside;
    let r = white + inside * inside; // + red
    let g = white + outside * inside; // + green
    let b = white + inside * outside; // + blue
    let expected_codes = [r, g, b].map(|linear| {
        let enc = if linear <= 0.003_130_8 {
            12.92 * linear
        } else {
            1.055 * linear.powf(1.0 / 2.4) - 0.055
        };
        (enc * 255.0).round() as i32
    });
    for channel in 0..3 {
        assert!(
            (expected_codes[channel] - i32::from(repeat_px[channel])).abs() <= 2,
            "bilinear+repeat corner expected {expected_codes:?}, got {repeat_px:?}"
        );
    }

    let path = reference_path("texture-sampling.png");
    compare_reference(
        &path,
        &ctx.adapter_info,
        width,
        height,
        &bilinear,
        &[
            ComparisonType::Mean(0.02),
            ComparisonType::Percentile {
                percentile: 0.99,
                threshold: 0.35,
            },
        ],
    );
}
