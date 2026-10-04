use foundations_verify::{
    ComparisonType, compare_reference, gpu_context, reference_path, render_and_readback,
};
use render_to_texture::sample::RenderToTexture;
use framework::{Gpu, Sample};

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

fn pixel(bytes: &[u8], width: u32, px: u32, py: u32) -> [u8; 4] {
    let offset = ((py * width + px) * 4) as usize;
    bytes[offset..offset + 4].try_into().unwrap()
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

/// Pixel bounds of the [-0.75, 0.75]^2 quad at the given target size.
fn quad_bounds(width: u32, height: u32) -> (u32, u32, u32, u32) {
    let x0 = ((-0.75 + 1.0) / 2.0 * width as f32).round() as u32;
    let x1 = ((0.75 + 1.0) / 2.0 * width as f32).round() as u32;
    let y0 = ((1.0 - 0.75) / 2.0 * height as f32).round() as u32;
    let y1 = ((1.0 + 0.75) / 2.0 * height as f32).round() as u32;
    (x0, y0, x1, y1)
}

/// Snapshot 30a: the RTT path must give the same frame as the direct path -
/// same reference, same corners, no half-texel shift.
#[test]
fn rtt_frame_matches_the_direct_path() {
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
    let mut sample = RenderToTexture::init(&gpu).expect("init sample");
    sample.resize(width, height);
    let bytes = render_and_readback(&ctx, width, height, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    let path = reference_path("render-to-texture.png");
    compare_reference(
        &path,
        &ctx.adapter_info,
        width,
        height,
        &bytes,
        &[
            // Byte-exact on the reference backend; slack absorbs half-LSB rounding.
            ComparisonType::Mean(0.005),
            ComparisonType::Percentile {
                percentile: 0.99,
                threshold: 0.02,
            },
        ],
    );

    // Four signed corners survive the detour through the intermediate texture.
    let (x0, y0, x1, y1) = quad_bounds(width, height);
    let (mid_x, mid_y) = ((x0 + x1) / 2, (y0 + y1) / 2);
    assert_color(
        pixel(&bytes, width, (x0 + mid_x) / 2, (y0 + mid_y) / 2),
        [255, 0, 0, 255],
        "top-left quadrant",
    );
    assert_color(
        pixel(&bytes, width, (mid_x + x1) / 2, (y0 + mid_y) / 2),
        [0, 255, 0, 255],
        "top-right quadrant",
    );
    assert_color(
        pixel(&bytes, width, (x0 + mid_x) / 2, (mid_y + y1) / 2),
        [0, 0, 255, 255],
        "bottom-left quadrant",
    );
    assert_color(
        pixel(&bytes, width, (mid_x + x1) / 2, (mid_y + y1) / 2),
        [255, 255, 255, 255],
        "bottom-right quadrant",
    );
    // Edge exactness: inside is quadrant color, two out is background.
    let gray = expected_code(128.0 / 255.0);
    assert_color(
        pixel(&bytes, width, x0, (y0 + mid_y) / 2),
        [255, 0, 0, 255],
        "inside the left edge",
    );
    assert_color(
        pixel(&bytes, width, x0 - 2, (y0 + mid_y) / 2),
        [gray, gray, gray, 255],
        "outside the left edge",
    );
    assert_color(
        pixel(&bytes, width, width / 2, 8),
        [gray, gray, gray, 255],
        "background",
    );

    // A second size exercises the recreate path at the new resolution.
    let (width, height) = (640, 512);
    sample.resize(width, height);
    let bytes = render_and_readback(&ctx, width, height, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    let (x0, y0, x1, y1) = quad_bounds(width, height);
    let (mid_x, mid_y) = ((x0 + x1) / 2, (y0 + y1) / 2);
    assert_color(
        pixel(&bytes, width, (x0 + mid_x) / 2, (y0 + mid_y) / 2),
        [255, 0, 0, 255],
        "resized top-left",
    );
    assert_color(
        pixel(&bytes, width, (mid_x + x1) / 2, (mid_y + y1) / 2),
        [255, 255, 255, 255],
        "resized bottom-right",
    );
}
