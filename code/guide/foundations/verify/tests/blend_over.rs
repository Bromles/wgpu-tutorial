use blend_over::params::{BLACK, BLUE, GREEN, RED, over_premultiplied, over_straight, premultiply};
use blend_over::sample::BlendOver;
use foundations_verify::{gpu_context, render_and_readback};
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
fn assert_color(actual: [u8; 4], expected: [u8; 4], label: impl std::fmt::Display) {
    for channel in 0..3 {
        assert!(
            (i32::from(expected[channel]) - i32::from(actual[channel])).abs() <= 1,
            "{label}: expected {} got {}",
            expected[channel],
            actual[channel]
        );
    }
}

/// Probe pixels of the design frame (x -2.4..2.4, y 1.8..-1.8): A control
/// (-1.2, 0), blue-only (-2.1, 0); B control (1.2, 0), red-only/green-only.
const A_CONTROL: (u32, u32) = (192, 288);
const A_BLUE: (u32, u32) = (48, 288);
const B_CONTROL: (u32, u32) = (576, 288);
const B_RED: (u32, u32) = (480, 288);
const B_GREEN: (u32, u32) = (672, 288);

/// Snapshot 27a: both representations compose the same linear mix; expected
/// = encode(linear mix).
#[test]
fn over_composes_the_same_pixel_in_both_representations() {
    let Some(ctx) = gpu_context() else {
        println!("skip: no suitable GPU adapter");
        return;
    };
    let gpu = Gpu {
        device: ctx.device.clone(),
        queue: ctx.queue.clone(),
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
    };
    let mut sample = BlendOver::init(&gpu).expect("init sample");

    // Pin the chapter numbers via the CPU model; both representations agree.
    let a_straight = over_straight(RED, BLUE);
    let a_premultiplied = over_premultiplied(premultiply(RED), premultiply(BLUE));
    let b_straight = over_straight(GREEN, over_straight(RED, BLACK));
    let b_premultiplied = over_premultiplied(
        premultiply(GREEN),
        over_premultiplied(premultiply(RED), premultiply(BLACK)),
    );
    for c in 0..3 {
        assert!((a_straight[c] - [0.5, 0.0, 0.5][c]).abs() < 1e-6);
        assert!((a_straight[c] - a_premultiplied[c]).abs() < 1e-6);
        assert!((b_straight[c] - [0.25, 0.5, 0.0][c]).abs() < 1e-6);
        assert!((b_straight[c] - b_premultiplied[c]).abs() < 1e-6);
    }

    for premultiplied in [false, true] {
        sample.set_representation(premultiplied);
        let bytes = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
            sample.draw(&gpu, encoder, view);
        });
        let label = if premultiplied {
            "premultiplied"
        } else {
            "straight"
        };
        // Preset A: red 0.5 over opaque blue -> (0.5, 0, 0.5).
        assert_color(
            pixel(&bytes, A_CONTROL.0, A_CONTROL.1),
            expected_color([0.5, 0.0, 0.5]),
            format!("A control, {label}"),
        );
        // The frame stays opaque: the background alpha survives the mix.
        assert_eq!(pixel(&bytes, A_CONTROL.0, A_CONTROL.1)[3], 255);
        // Preset B: red 0.5 over opaque black, green 0.5 on top -> (0.25, 0.5, 0).
        assert_color(
            pixel(&bytes, B_CONTROL.0, B_CONTROL.1),
            expected_color([0.25, 0.5, 0.0]),
            format!("B control, {label}"),
        );
        // Uncovered zones pin the inputs: pure blue, red-only, green-only.
        assert_color(
            pixel(&bytes, A_BLUE.0, A_BLUE.1),
            expected_color([0.0, 0.0, 1.0]),
            format!("A blue, {label}"),
        );
        assert_color(
            pixel(&bytes, B_RED.0, B_RED.1),
            expected_color([0.5, 0.0, 0.0]),
            format!("B red only, {label}"),
        );
        assert_color(
            pixel(&bytes, B_GREEN.0, B_GREEN.1),
            expected_color([0.0, 0.5, 0.0]),
            format!("B green only, {label}"),
        );
    }
}
