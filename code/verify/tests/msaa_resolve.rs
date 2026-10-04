use verify::{gpu_context, render_and_readback};
use msaa_resolve::sample::MsaaResolve;
use framework::{Gpu, Sample};

const WIDTH: u32 = 768;
const HEIGHT: u32 = 576;

/// The clear color of the scene pass, linear, as in the sample.
const CLEAR: [f32; 3] = [0.1, 0.1, 0.14];

fn srgb_encode(x: f32) -> f32 {
    if x <= 0.003_130_8 {
        12.92 * x
    } else {
        1.055 * x.powf(1.0 / 2.4) - 0.055
    }
}

/// Full storage path: quantize to UNORM on store, encode to sRGB on the surface.
fn expected_code(linear: f32) -> u8 {
    let stored = (linear.clamp(0.0, 1.0) * 255.0).round() / 255.0;
    (srgb_encode(stored) * 255.0).round() as u8
}

fn pixel(bytes: &[u8], px: u32, py: u32) -> [u8; 4] {
    let offset = ((py * WIDTH + px) * 4) as usize;
    bytes[offset..offset + 4].try_into().unwrap()
}

fn is_pure_background(px: [u8; 4]) -> bool {
    (0..3).all(|c| (i32::from(px[c]) - i32::from(expected_code(CLEAR[c]))).abs() <= 2)
}

/// Pixel centre nearest the projection of a world point (fixed camera:
/// eye (0, 0, 5), fov 60, target the origin).
fn project(x: f32, y: f32, z: f32) -> (u32, u32) {
    let z_dist = 5.0 - z;
    let half_tan = std::f32::consts::FRAC_PI_6.tan();
    let aspect = WIDTH as f32 / HEIGHT as f32;
    let ndc_x = x / (z_dist * half_tan * aspect);
    let ndc_y = y / (z_dist * half_tan);
    let px = ((ndc_x + 1.0) / 2.0 * WIDTH as f32 - 0.5)
        .round()
        .clamp(0.0, WIDTH as f32 - 1.0);
    let py = ((1.0 - ndc_y) / 2.0 * HEIGHT as f32 - 0.5)
        .round()
        .clamp(0.0, HEIGHT as f32 - 1.0);
    (px as u32, py as u32)
}

/// Snapshot 32: at count = 4 the diagonal silhouette gains mixed pixels
/// (partial coverage becomes intermediate colors); interiors stay identical.
#[test]
fn resolve_adds_mixed_pixels_along_the_silhouette() {
    let Some(ctx) = gpu_context() else {
        println!("skip: no suitable GPU adapter");
        return;
    };
    let gpu = Gpu {
        device: ctx.device.clone(),
        queue: ctx.queue.clone(),
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
    };
    let mut sample = MsaaResolve::init(&gpu).expect("init sample");
    sample.resize(WIDTH, HEIGHT);

    sample.set_msaa(false);
    let single = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    sample.set_msaa(true);
    let multi = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });

    // Interiors: total coverage, both modes agree byte for byte.
    let blue = project(0.3, 1.0, 0.85);
    let red = project(-0.3, 1.0, 0.0);
    for (point, linear) in [(blue, [0.25_f32, 0.4, 0.85]), (red, [0.85_f32, 0.25, 0.25])] {
        let (px, py) = point;
        let expected: [u8; 3] = linear.map(expected_code);
        for bytes in [&single, &multi] {
            let actual = pixel(bytes, px, py);
            for channel in 0..3 {
                assert!(
                    (i32::from(expected[channel]) - i32::from(actual[channel])).abs() <= 1,
                    "interior ({px},{py}) channel {channel}: expected {} got {}",
                    expected[channel],
                    actual[channel]
                );
            }
        }
    }

    // Rows through the blue silhouette: count=1 keeps more pure background.
    let rows = 75..101u32;
    let edge_window = 400..645u32;
    let mut single_background = 0usize;
    let mut multi_background = 0usize;
    let mut mixed_by_resolve = 0usize;
    for py in rows {
        for px in edge_window.clone() {
            let one = pixel(&single, px, py);
            let four = pixel(&multi, px, py);
            if is_pure_background(one) {
                single_background += 1;
            }
            if is_pure_background(four) {
                multi_background += 1;
            }
            let differs = (0..3).any(|c| (i32::from(one[c]) - i32::from(four[c])).abs() > 8);
            if differs {
                mixed_by_resolve += 1;
            }
        }
    }
    assert!(
        mixed_by_resolve > 0,
        "count = 4 must mix pixels along the silhouette"
    );
    assert!(
        single_background > multi_background,
        "count = 1 keeps more pure background than count = 4: {single_background} vs {multi_background}"
    );

    // Far from any edge nothing may change: clean background in both modes.
    for py in 75..101u32 {
        for px in 500..620u32 {
            assert!(
                is_pure_background(pixel(&single, px, py))
                    && is_pure_background(pixel(&multi, px, py)),
                "({px},{py}) must stay background away from the edge"
            );
        }
    }
}
