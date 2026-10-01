use compute_basics::sample::{CELLS, ComputeBasics};
use foundations_verify::{gpu_context, render_and_readback};
use shell::{Gpu, Sample};

const WIDTH: u32 = 768;
const HEIGHT: u32 = 576;

// Strip frame shared with the constants of strip.wgsl.
const X0: f32 = -0.9;
const STRIP_WIDTH: f32 = 1.8;
const Y0: f32 = -0.95;
const STRIP_HEIGHT: f32 = 0.55;

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

fn background() -> [u8; 4] {
    [
        expected_code(0.08),
        expected_code(0.08),
        expected_code(0.12),
        255,
    ]
}

fn pixel(bytes: &[u8], px: u32, py: u32) -> [u8; 4] {
    let offset = ((py * WIDTH + px) * 4) as usize;
    bytes[offset..offset + 4].try_into().unwrap()
}

/// Pixel column near the middle of strip cell k.
fn cell_x(k: u32) -> u32 {
    let left = (X0 + k as f32 * STRIP_WIDTH / CELLS as f32 + 1.0) / 2.0 * WIDTH as f32;
    let right = (X0 + (k + 1) as f32 * STRIP_WIDTH / CELLS as f32 + 1.0) / 2.0 * WIDTH as f32;
    ((left + right) / 2.0).floor() as u32
}

/// Pixel row at the given fraction of the filled height of a cell with
/// value `value`.
fn height_row(value: f32, fraction: f32) -> u32 {
    let y = Y0 + fraction * value * STRIP_HEIGHT;
    ((1.0 - y) / 2.0 * HEIGHT as f32).floor() as u32
}

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

fn gray(code: u8) -> [u8; 4] {
    [code, code, code, 255]
}

/// Snapshot 29: values stay on the GPU, so the check reads the strip: the
/// compute pass fills i / (count - 1), and the R repeat switches the denominator.
#[test]
fn compute_values_show_in_strip() {
    let Some(ctx) = gpu_context() else {
        println!("skip: no suitable GPU adapter");
        return;
    };
    let gpu = Gpu {
        device: ctx.device.clone(),
        queue: ctx.queue.clone(),
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
    };
    let mut sample = ComputeBasics::init(&gpu).expect("init sample");

    // 257 values: 5 workgroups of 64 launched, 63 invocations discarded.
    let bytes = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    let row = height_row(0.5, 0.3);
    // Value 0: zero height, the background stays visible.
    assert_color(pixel(&bytes, cell_x(0), row), background(), "cell 0 empty");
    // Value 0.5: gray 188 at 30% of the filled height.
    assert_color(
        pixel(&bytes, cell_x(128), row),
        gray(expected_code(0.5)),
        "cell 128 half",
    );
    // Above the cell top the background returns: height matches value.
    let above = height_row(0.5, 1.2);
    assert_color(
        pixel(&bytes, cell_x(128), above),
        background(),
        "cell 128 above top",
    );
    // Value 1: full height, gray 255.
    let full_row = height_row(1.0, 0.9);
    assert_color(
        pixel(&bytes, cell_x(256), full_row),
        gray(expected_code(1.0)),
        "cell 256 full",
    );

    // The R repeat: 256 values, 4 workgroups, i / 255, cell 256 empty.
    sample.set_count(256);
    let bytes = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    assert_color(
        pixel(&bytes, cell_x(128), row),
        gray(expected_code(128.0 / 255.0)),
        "cell 128 half after R",
    );
    assert_color(
        pixel(&bytes, cell_x(255), full_row),
        gray(expected_code(1.0)),
        "cell 255 full after R",
    );
    assert_color(
        pixel(&bytes, cell_x(256), full_row),
        background(),
        "cell 256 hidden after R",
    );
}
