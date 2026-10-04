use blend_order::sample::{BlendOrder, Order};
use verify::{gpu_context, render_and_readback};
use framework::{Gpu, Sample};

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

fn expected_color(linear: [f32; 3]) -> [u8; 4] {
    [
        expected_code(linear[0]),
        expected_code(linear[1]),
        expected_code(linear[2]),
        255,
    ]
}

fn pixel(bytes: &[u8], px: u32, py: u32) -> [u8; 4] {
    let offset = ((py * WIDTH + px) * 4) as usize;
    bytes[offset..offset + 4].try_into().unwrap()
}

/// Probes one code away from the hardware sRGB encode.
fn assert_color(actual: [u8; 4], expected: [u8; 4], label: &str) {
    for channel in 0..3 {
        assert!(
            (i32::from(expected[channel]) - i32::from(actual[channel])).abs() <= 1,
            "{label}: expected {} got {}",
            expected[channel],
            actual[channel]
        );
    }
}

/// Probe pixels of the design frame (x -2.4..2.4, y 1.8..-1.8): control at
/// (-0.6, 0), red-only (-1.2, 0), green-only (0, 0); cutout probes at y = +-0.45.
const CONTROL: (u32, u32) = (288, 288);
const RED_ONLY_UPPER: (u32, u32) = (192, 216);
const RED_ONLY_LOWER: (u32, u32) = (192, 360);
const GREEN_ONLY_UPPER: (u32, u32) = (384, 216);
const GREEN_ONLY_LOWER: (u32, u32) = (384, 360);
const OVERLAP_LOWER: (u32, u32) = (288, 360);

/// Snapshot 27b: `over` is not commutative, so the O key flips the control
/// pixel between (0.25, 0.5, 0) and (0.5, 0.25, 0).
#[test]
fn blend_order_flips_the_control_pixel() {
    let Some(ctx) = gpu_context() else {
        println!("skip: no suitable GPU adapter");
        return;
    };
    let gpu = Gpu {
        device: ctx.device.clone(),
        queue: ctx.queue.clone(),
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
    };
    let mut sample = BlendOrder::init(&gpu).expect("init sample");
    sample.resize(WIDTH, HEIGHT);

    for premultiplied in [false, true] {
        sample.set_representation(premultiplied);
        let label = if premultiplied {
            "premultiplied"
        } else {
            "straight"
        };
        sample.set_order(Order::RedFirst);
        let bytes = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
            sample.draw(&gpu, encoder, view);
        });
        assert_color(
            pixel(&bytes, CONTROL.0, CONTROL.1),
            expected_color([0.25, 0.5, 0.0]),
            format!("{label}, red first").as_str(),
        );
        sample.set_order(Order::GreenFirst);
        let bytes = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
            sample.draw(&gpu, encoder, view);
        });
        assert_color(
            pixel(&bytes, CONTROL.0, CONTROL.1),
            expected_color([0.5, 0.25, 0.0]),
            format!("{label}, green first").as_str(),
        );
    }
}

/// Snapshot 27b, cutout: alpha below the threshold discards; above it passes
/// and writes depth - at equal depth the first draw survives under `Less`.
#[test]
fn cutout_threshold_decides_depth_and_color() {
    let Some(ctx) = gpu_context() else {
        println!("skip: no suitable GPU adapter");
        return;
    };
    let gpu = Gpu {
        device: ctx.device.clone(),
        queue: ctx.queue.clone(),
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
    };
    let mut sample = BlendOrder::init(&gpu).expect("init sample");
    sample.resize(WIDTH, HEIGHT);
    sample.set_cutout(true);

    for (order, overlap) in [
        (Order::RedFirst, expected_color([1.0, 0.0, 0.0])),
        (Order::GreenFirst, expected_color([0.0, 1.0, 0.0])),
    ] {
        sample.set_order(order);
        let bytes = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
            sample.draw(&gpu, encoder, view);
        });
        let label = match order {
            Order::RedFirst => "red first",
            Order::GreenFirst => "green first",
        };
        // Upper halves: alpha 0.49 discards; background and depth survive.
        assert_color(
            pixel(&bytes, RED_ONLY_UPPER.0, RED_ONLY_UPPER.1),
            expected_color([0.0; 3]),
            format!("{label}, red upper").as_str(),
        );
        assert_color(
            pixel(&bytes, GREEN_ONLY_UPPER.0, GREEN_ONLY_UPPER.1),
            expected_color([0.0; 3]),
            format!("{label}, green upper").as_str(),
        );
        // Lower halves: alpha 0.51 passes and writes the full source color.
        assert_color(
            pixel(&bytes, RED_ONLY_LOWER.0, RED_ONLY_LOWER.1),
            expected_color([1.0, 0.0, 0.0]),
            format!("{label}, red lower").as_str(),
        );
        assert_color(
            pixel(&bytes, GREEN_ONLY_LOWER.0, GREEN_ONLY_LOWER.1),
            expected_color([0.0, 1.0, 0.0]),
            format!("{label}, green lower").as_str(),
        );
        // Overlap, lower half: equal depth, first draw's write rejects the second.
        assert_color(
            pixel(&bytes, OVERLAP_LOWER.0, OVERLAP_LOWER.1),
            overlap,
            format!("{label}, overlap lower").as_str(),
        );
    }
}
