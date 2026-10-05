use wgpu::TextureFormat;
use framework::{Gpu, Sample};
use depth_culling::sample::{Culling, DepthCulling, Order};
use verify::{gpu_context, render_and_readback};

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

fn triangle_color(linear: [f32; 3]) -> [u8; 4] {
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

fn count_color(bytes: &[u8], color: [u8; 4]) -> usize {
    bytes
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|c| {
            (0..3).all(|channel| (i32::from(color[channel]) - i32::from(c[channel])).abs() <= 1)
        })
        .count()
}

/// Probes on the NDC y = 0.5 row either side of the crossing (NDC x = 0):
/// left is red-only, right is covered by both with blue nearer.
const LEFT_PROBE: (u32, u32) = (337, 144);
const RIGHT_PROBE: (u32, u32) = (422, 144);

/// Snapshot 20: with depth on, swapping the draw order must not change a
/// pixel - the attachment decides which surface survives.
#[test]
fn with_depth_the_draw_order_does_not_change_pixels() {
    let Some(ctx) = gpu_context() else {
        println!("skip: no suitable GPU adapter");
        return;
    };
    let gpu = Gpu {
        device: ctx.device.clone(),
        queue: ctx.queue.clone(),
        format: TextureFormat::Rgba8UnormSrgb,
    };
    let mut sample = DepthCulling::init(&gpu).expect("init sample");
    sample.resize(WIDTH, HEIGHT);
    sample.set_depth_enabled(true);
    sample.set_culling(Culling::Off);

    sample.set_order(Order::RedFirst);
    let red_first = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    sample.set_order(Order::BlueFirst);
    let blue_first = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });

    // Equal-depth fragments keep the first draw under `Less`; a few are fine.
    let mut differing = 0;
    for (a, b) in red_first
        .as_chunks::<4>()
        .0
        .iter()
        .zip(blue_first.as_chunks::<4>().0)
    {
        if a != b {
            differing += 1;
        }
    }
    assert!(
        differing <= 4,
        "{differing} pixels depend on the draw order"
    );

    let red = triangle_color([0.85, 0.25, 0.25]);
    let blue = triangle_color([0.25, 0.4, 0.85]);
    assert_color(
        pixel(&red_first, LEFT_PROBE.0, LEFT_PROBE.1),
        red,
        "left probe, red first",
    );
    assert_color(
        pixel(&red_first, RIGHT_PROBE.0, RIGHT_PROBE.1),
        blue,
        "right probe, red first",
    );
    assert_color(
        pixel(&blue_first, LEFT_PROBE.0, LEFT_PROBE.1),
        red,
        "left probe, blue first",
    );
    assert_color(
        pixel(&blue_first, RIGHT_PROBE.0, RIGHT_PROBE.1),
        blue,
        "right probe, blue first",
    );
}

/// Without depth the last draw covers whatever was there: the right probe
/// flips with the order - exactly the problem the attachment solves.
#[test]
fn without_depth_the_last_draw_wins() {
    let Some(ctx) = gpu_context() else {
        println!("skip: no suitable GPU adapter");
        return;
    };
    let gpu = Gpu {
        device: ctx.device.clone(),
        queue: ctx.queue.clone(),
        format: TextureFormat::Rgba8UnormSrgb,
    };
    let mut sample = DepthCulling::init(&gpu).expect("init sample");
    sample.resize(WIDTH, HEIGHT);
    sample.set_depth_enabled(false);
    sample.set_culling(Culling::Off);

    let red = triangle_color([0.85, 0.25, 0.25]);
    let blue = triangle_color([0.25, 0.4, 0.85]);

    sample.set_order(Order::RedFirst);
    let bytes = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    assert_color(
        pixel(&bytes, LEFT_PROBE.0, LEFT_PROBE.1),
        red,
        "left probe, red first",
    );
    assert_color(
        pixel(&bytes, RIGHT_PROBE.0, RIGHT_PROBE.1),
        blue,
        "right probe, red first",
    );

    sample.set_order(Order::BlueFirst);
    let bytes = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    assert_color(
        pixel(&bytes, LEFT_PROBE.0, LEFT_PROBE.1),
        red,
        "left probe, blue first",
    );
    assert_color(
        pixel(&bytes, RIGHT_PROBE.0, RIGHT_PROBE.1),
        red,
        "right probe, blue first",
    );
}

/// Both triangles are wound CCW for this camera, so culling front faces
/// empties the frame; flipping the blue winding brings exactly it back.
#[test]
fn front_culling_drops_counter_clockwise_triangles() {
    let Some(ctx) = gpu_context() else {
        println!("skip: no suitable GPU adapter");
        return;
    };
    let gpu = Gpu {
        device: ctx.device.clone(),
        queue: ctx.queue.clone(),
        format: TextureFormat::Rgba8UnormSrgb,
    };
    let mut sample = DepthCulling::init(&gpu).expect("init sample");
    sample.resize(WIDTH, HEIGHT);
    sample.set_depth_enabled(true);
    sample.set_culling(Culling::Front);

    let red = triangle_color([0.85, 0.25, 0.25]);
    let blue = triangle_color([0.25, 0.4, 0.85]);

    sample.set_flipped_winding(false);
    let bytes = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    assert_eq!(count_color(&bytes, red), 0, "red triangle must be culled");
    assert_eq!(count_color(&bytes, blue), 0, "blue triangle must be culled");

    sample.set_flipped_winding(true);
    let bytes = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    assert_eq!(count_color(&bytes, red), 0, "red triangle must stay culled");
    assert!(
        count_color(&bytes, blue) > 1000,
        "the flipped blue triangle must become visible"
    );
}
