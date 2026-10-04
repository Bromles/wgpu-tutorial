use blinn_phong::sample::BlinnPhong;
use verify::{gpu_context, render_and_readback};
use glam::Vec3;
use framework::{Gpu, Sample};

const WIDTH: u32 = 768;
const HEIGHT: u32 = 576;

const ALBEDO: f32 = 0.5;
const SPECULAR: f32 = 0.7;
const AMBIENT: f32 = 0.1;
const INTENSITY: f32 = 0.6;
const SHININESS: f32 = 32.0;

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

/// World point of the z = 0 plane under a pixel centre, camera at `eye`
/// aimed at the origin, fov 60 - matches the shader's interpolated position.
fn plane_point(eye: Vec3, px: u32, py: u32) -> Vec3 {
    let forward = -eye.normalize();
    let right = forward.cross(Vec3::Y).normalize();
    let up = right.cross(forward).normalize();
    let ndc_x = (px as f32 + 0.5) / WIDTH as f32 * 2.0 - 1.0;
    let ndc_y = 1.0 - 2.0 * (py as f32 + 0.5) / HEIGHT as f32;
    let half_tan = (std::f32::consts::FRAC_PI_6).tan();
    let aspect = WIDTH as f32 / HEIGHT as f32;
    let dir = (right * (ndc_x * half_tan * aspect) + up * (ndc_y * half_tan) - forward).normalize();
    let t = -eye.z / dir.z;
    eye + dir * t
}

/// The Blinn-Phong formula mirrored on the CPU: diffuse plus highlight.
fn shade(p: Vec3, eye: Vec3) -> u8 {
    let n = Vec3::Z;
    let l = Vec3::Z;
    let v = (eye - p).normalize();
    // The shader guards L + V ~ 0 with a select; presets never hit it.
    let h = (l + v).normalize();
    let diffuse = n.dot(l).max(0.0);
    let spec = n.dot(h).clamp(0.0, 1.0).powf(SHININESS);
    let linear = ALBEDO * (AMBIENT + INTENSITY * diffuse) + SPECULAR * (INTENSITY * spec);
    expected_code(linear)
}

/// Snapshot 25: at the frame centre V = L, so N*H peaks; off centre and
/// after the camera moves, the highlight follows the half-vector model.
#[test]
fn highlight_follows_the_half_vector() {
    let Some(ctx) = gpu_context() else {
        println!("skip: no suitable GPU adapter");
        return;
    };
    let gpu = Gpu {
        device: ctx.device.clone(),
        queue: ctx.queue.clone(),
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
    };
    let mut sample = BlinnPhong::init(&gpu).expect("init sample");
    sample.resize(WIDTH, HEIGHT);

    let eye = Vec3::new(0.0, 0.0, 3.0);
    let bytes = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    // Centre: full highlight on top of the full diffuse term.
    let center = pixel(&bytes, WIDTH / 2, HEIGHT / 2);
    let expected_center = shade(plane_point(eye, WIDTH / 2, HEIGHT / 2), eye);
    assert!(
        (i32::from(expected_center) - i32::from(center[0])).abs() <= 1,
        "centre: expected {expected_center} got {}",
        center[0]
    );
    // Off centre: V tilts away from L, N*H < 1, the highlight falls off.
    for (px, py) in [(250, 200), (500, 380), (672, 288)] {
        let actual = pixel(&bytes, px, py);
        let expected = shade(plane_point(eye, px, py), eye);
        assert!(
            (i32::from(expected) - i32::from(actual[0])).abs() <= 1,
            "({px},{py}): expected {expected} got {}",
            actual[0]
        );
        assert!(
            actual[0] < center[0],
            "({px},{py}) must be dimmer than the centre"
        );
    }

    // Moved camera: centre still the origin but V != L, brightness drops.
    sample.set_camera(1);
    let eye_moved = Vec3::new(1.5, 0.0, 3.0);
    let bytes = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    let center_moved = pixel(&bytes, WIDTH / 2, HEIGHT / 2);
    let expected_moved = shade(plane_point(eye_moved, WIDTH / 2, HEIGHT / 2), eye_moved);
    assert!(
        (i32::from(expected_moved) - i32::from(center_moved[0])).abs() <= 1,
        "moved camera centre: expected {expected_moved} got {}",
        center_moved[0]
    );
    assert!(
        center_moved[0] < center[0],
        "the highlight must move with the eye, not stay at the origin"
    );
}
