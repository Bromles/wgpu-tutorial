use foundations_verify::{gpu_context, render_and_readback};
use image_pipeline::sample::{ImagePipeline, OutputMode};
use shell::{Gpu, Sample};

const WIDTH: u32 = 768;
const HEIGHT: u32 = 576;

fn srgb_encode(x: f32) -> f32 {
    if x <= 0.003_130_8 {
        12.92 * x
    } else {
        1.055 * x.powf(1.0 / 2.4) - 0.055
    }
}

fn expected_code(linear: f32) -> u8 {
    (srgb_encode(linear.clamp(0.0, 1.0)) * 255.0).round() as u8
}

fn pixel(bytes: &[u8], px: u32, py: u32) -> [u8; 4] {
    let offset = ((py * WIDTH + px) * 4) as usize;
    bytes[offset..offset + 4].try_into().unwrap()
}

/// Snapshot 30b: fragment and compute forks do the same per-pixel operation,
/// so the difference is zero byte for byte and both equal encode(0.5 * linear).
#[test]
fn fragment_and_compute_halving_agree() {
    let Some(ctx) = gpu_context() else {
        println!("skip: no suitable GPU adapter");
        return;
    };
    let gpu = Gpu {
        device: ctx.device.clone(),
        queue: ctx.queue.clone(),
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
    };
    let mut sample = ImagePipeline::init(&gpu).expect("init sample");
    sample.resize(WIDTH, HEIGHT);

    sample.set_mode(OutputMode::Fragment);
    let fragment = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    sample.set_mode(OutputMode::Compute);
    let compute = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });

    // The absolute screen difference of the two forks is zero.
    assert_eq!(fragment, compute, "fragment and compute results differ");

    // Input: the chapter 13a frame in linear RGBA8Unorm, probed away from edges.
    struct Region {
        label: &'static str,
        x_begin: u32,
        y_begin: u32,
        x_end: u32,
        y_end: u32,
        rgb: [u8; 3],
    }
    let regions = [
        Region {
            label: "red",
            x_begin: 100,
            y_begin: 76,
            x_end: 380,
            y_end: 284,
            rgb: [255, 0, 0],
        },
        Region {
            label: "green",
            x_begin: 388,
            y_begin: 76,
            x_end: 668,
            y_end: 284,
            rgb: [0, 255, 0],
        },
        Region {
            label: "blue",
            x_begin: 100,
            y_begin: 292,
            x_end: 380,
            y_end: 500,
            rgb: [0, 0, 255],
        },
        Region {
            label: "white",
            x_begin: 388,
            y_begin: 292,
            x_end: 668,
            y_end: 500,
            rgb: [255, 255, 255],
        },
        Region {
            label: "top margin",
            x_begin: 0,
            y_begin: 8,
            x_end: WIDTH,
            y_end: 64,
            rgb: [128, 128, 128],
        },
        Region {
            label: "bottom margin",
            x_begin: 0,
            y_begin: 512,
            x_end: WIDTH,
            y_end: 568,
            rgb: [128, 128, 128],
        },
        Region {
            label: "left margin",
            x_begin: 8,
            y_begin: 0,
            x_end: 88,
            y_end: HEIGHT,
            rgb: [128, 128, 128],
        },
        Region {
            label: "right margin",
            x_begin: 680,
            y_begin: 0,
            x_end: 760,
            y_end: HEIGHT,
            rgb: [128, 128, 128],
        },
    ];
    let mut checked = 0usize;
    for Region {
        label,
        x_begin,
        y_begin,
        x_end,
        y_end,
        rgb: input,
    } in regions
    {
        for py in y_begin..y_end {
            for px in x_begin..x_end {
                let actual = pixel(&fragment, px, py);
                for channel in 0..3 {
                    // Input byte is linear; the fork halves it, the surface encodes once.
                    let linear = f32::from(input[channel]) / 255.0;
                    let expected = expected_code(0.5 * linear);
                    assert!(
                        (i32::from(expected) - i32::from(actual[channel])).abs() <= 1,
                        "{label} ({px},{py}) channel {channel}: expected {expected} got {}",
                        actual[channel]
                    );
                }
                assert_eq!(
                    actual[3], 255,
                    "{label} ({px},{py}): alpha must pass through"
                );
                checked += 1;
            }
        }
    }
    assert!(
        checked >= 100_000,
        "expected full-region coverage, got {checked}"
    );
}
