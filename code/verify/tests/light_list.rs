use verify::{gpu_context, render_and_readback};
use light_list::params::shade;
use light_list::sample::LightList;
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

fn pixel(bytes: &[u8], px: u32, py: u32) -> [u8; 4] {
    let offset = ((py * WIDTH + px) * 4) as usize;
    bytes[offset..offset + 4].try_into().unwrap()
}

/// World point of the z = 0 plane under a pixel centre, camera at (0, 0, 3).
fn plane_point(px: u32, py: u32) -> glam::Vec3 {
    let ndc_x = (px as f32 + 0.5) / WIDTH as f32 * 2.0 - 1.0;
    let ndc_y = 1.0 - 2.0 * (py as f32 + 0.5) / HEIGHT as f32;
    let half_h = (std::f32::consts::FRAC_PI_6).tan() * 3.0;
    let half_w = half_h * (WIDTH as f32 / HEIGHT as f32);
    glam::Vec3::new(ndc_x * half_w, ndc_y * half_h, 0.0)
}

/// Snapshot 26b: the storage loop sums linear contributions - two identical
/// sources double the light in linear units; zero sources leave ambient alone.
#[test]
fn light_list_sums_linear_contributions() {
    let Some(ctx) = gpu_context() else {
        println!("skip: no suitable GPU adapter");
        return;
    };
    let gpu = Gpu {
        device: ctx.device.clone(),
        queue: ctx.queue.clone(),
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
    };
    let mut sample = LightList::init(&gpu).expect("init sample");
    sample.resize(WIDTH, HEIGHT);

    let center = plane_point(WIDTH / 2, HEIGHT / 2);
    // Pin the chapter numbers via the CPU model: one light -> 0.2, two -> 0.4.
    assert_eq!(
        expected_code(shade(center, glam::Vec3::Z, 1, 0.0)),
        expected_code(0.2)
    );
    assert_eq!(
        expected_code(shade(center, glam::Vec3::Z, 2, 0.0)),
        expected_code(0.4)
    );

    for count in [1u32, 2, 3] {
        sample.set_count(count);
        let bytes = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
            sample.draw(&gpu, encoder, view);
        });
        let actual = pixel(&bytes, WIDTH / 2, HEIGHT / 2);
        let expected = expected_code(shade(center, glam::Vec3::Z, count, 0.0));
        assert!(
            (i32::from(expected) - i32::from(actual[0])).abs() <= 1,
            "count {count}: expected {expected} got {}",
            actual[0]
        );
    }

    // Doubling the light doubles the linear value, not the sRGB code.
    let one = expected_code(shade(center, glam::Vec3::Z, 1, 0.0));
    let two = expected_code(shade(center, glam::Vec3::Z, 2, 0.0));
    assert!(
        two > one && two < 2 * one,
        "codes are not linear: {one} -> {two}"
    );

    // Zero lights: ambient only, and the chapter ambient is 0.
    sample.set_count(0);
    let bytes = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    let actual = pixel(&bytes, WIDTH / 2, HEIGHT / 2);
    assert_eq!(actual[0], 0, "no lights and no ambient leave black");
}
