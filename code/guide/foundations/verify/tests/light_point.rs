use foundations_verify::{gpu_context, render_and_readback};
use glam::Vec3;
use light_point::sample::LightPoint;
use shell::{Gpu, Sample};

const WIDTH: u32 = 768;
const HEIGHT: u32 = 576;

const POWER: f32 = 0.4;
const ALBEDO: f32 = 0.5;
const R_MIN: f32 = 0.1;
const SPOT_DIR: Vec3 = Vec3::new(0.0, 0.0, -1.0);
const SPOT_COS_INNER: f32 = 0.9659258;
const SPOT_COS_OUTER: f32 = 0.8660254;

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
fn plane_point(px: u32, py: u32) -> Vec3 {
    let ndc_x = (px as f32 + 0.5) / WIDTH as f32 * 2.0 - 1.0;
    let ndc_y = 1.0 - 2.0 * (py as f32 + 0.5) / HEIGHT as f32;
    let half_h = (std::f32::consts::FRAC_PI_6).tan() * 3.0;
    let half_w = half_h * (WIDTH as f32 / HEIGHT as f32);
    Vec3::new(ndc_x * half_w, ndc_y * half_h, 0.0)
}

/// Chapter formula mirrored on the CPU: attenuation plus optional cone fade.
fn shade(p: Vec3, light_pos: Vec3, spot_on: bool) -> u8 {
    let to_light = light_pos - p;
    let r = to_light.length();
    let l = to_light / r;
    let attenuation = 1.0 / (r * r).max(R_MIN * R_MIN);
    let mut c = POWER * attenuation * Vec3::Z.dot(l).max(0.0);
    if spot_on {
        let d = (-l).dot(SPOT_DIR);
        let t = ((d - SPOT_COS_OUTER) / (SPOT_COS_INNER - SPOT_COS_OUTER)).clamp(0.0, 1.0);
        c *= t;
    }
    expected_code(ALBEDO * c)
}

/// Snapshot 26a: the direct term falls as 1/r^2 - r = 1 gives the linear
/// 0.2, r = 2 gives 0.05 - and oblique points lose the N*L factor too.
#[test]
fn point_light_attenuates_with_distance() {
    let Some(ctx) = gpu_context() else {
        println!("skip: no suitable GPU adapter");
        return;
    };
    let gpu = Gpu {
        device: ctx.device.clone(),
        queue: ctx.queue.clone(),
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
    };
    let mut sample = LightPoint::init(&gpu).expect("init sample");
    sample.resize(WIDTH, HEIGHT);

    // Passport numbers pinned first: straight under the source.
    assert_eq!(
        shade(Vec3::ZERO, Vec3::new(0.0, 0.0, 1.0), false),
        expected_code(0.2)
    );
    assert_eq!(
        shade(Vec3::ZERO, Vec3::new(0.0, 0.0, 2.0), false),
        expected_code(0.05)
    );

    // r = 1 straight under the source at height 1.
    sample.set_light_pos(Vec3::new(0.0, 0.0, 1.0));
    let bytes = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    let center = pixel(&bytes, WIDTH / 2, HEIGHT / 2);
    let expected = shade(
        plane_point(WIDTH / 2, HEIGHT / 2),
        Vec3::new(0.0, 0.0, 1.0),
        false,
    );
    assert!(
        (i32::from(expected) - i32::from(center[0])).abs() <= 1,
        "r = 1 centre: expected {expected} got {}",
        center[0]
    );

    // r = 2 straight under the source at height 2.
    sample.set_light_pos(Vec3::new(0.0, 0.0, 2.0));
    let bytes = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    let center = pixel(&bytes, WIDTH / 2, HEIGHT / 2);
    let expected = shade(
        plane_point(WIDTH / 2, HEIGHT / 2),
        Vec3::new(0.0, 0.0, 2.0),
        false,
    );
    assert!(
        (i32::from(expected) - i32::from(center[0])).abs() <= 1,
        "r = 2 centre: expected {expected} got {}",
        center[0]
    );

    // Oblique point at r = 2 from the height-1 source: N*L < 1 too.
    sample.set_light_pos(Vec3::new(0.0, 0.0, 1.0));
    let bytes = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    let oblique = pixel(&bytes, 672, HEIGHT / 2);
    let expected = shade(
        plane_point(672, HEIGHT / 2),
        Vec3::new(0.0, 0.0, 1.0),
        false,
    );
    assert!(
        (i32::from(expected) - i32::from(oblique[0])).abs() <= 1,
        "oblique r = 2: expected {expected} got {}",
        oblique[0]
    );
}

/// The cone keeps the axis at full brightness and cuts the rim to zero.
#[test]
fn spot_cone_masks_the_rim_not_the_axis() {
    let Some(ctx) = gpu_context() else {
        println!("skip: no suitable GPU adapter");
        return;
    };
    let gpu = Gpu {
        device: ctx.device.clone(),
        queue: ctx.queue.clone(),
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
    };
    let mut sample = LightPoint::init(&gpu).expect("init sample");
    sample.resize(WIDTH, HEIGHT);
    sample.set_light_pos(Vec3::new(0.0, 0.0, 1.0));

    sample.set_spot_on(true);
    let coned = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    // On the axis the fade factor is 1: the centre keeps its full code.
    let center = pixel(&coned, WIDTH / 2, HEIGHT / 2);
    let expected_center = shade(
        plane_point(WIDTH / 2, HEIGHT / 2),
        Vec3::new(0.0, 0.0, 1.0),
        true,
    );
    assert!(
        (i32::from(expected_center) - i32::from(center[0])).abs() <= 1,
        "cone centre: expected {expected_center} got {}",
        center[0]
    );
    // Far off-axis the ray is outside the outer cone: linear 0, code 0.
    let rim = pixel(&coned, 100, HEIGHT / 2);
    let expected_rim = shade(plane_point(100, HEIGHT / 2), Vec3::new(0.0, 0.0, 1.0), true);
    assert_eq!(expected_rim, 0);
    assert!(rim[0] <= 1, "cone rim must be black, got {}", rim[0]);

    // Without the cone the same rim pixel is lit: the mask is the difference.
    sample.set_spot_on(false);
    let plain = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    let rim_plain = pixel(&plain, 100, HEIGHT / 2);
    assert!(rim_plain[0] > rim[0], "the cone must darken the rim");
}
