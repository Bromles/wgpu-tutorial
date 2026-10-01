use foundations_verify::{gpu_context, render_and_readback};
use glam::{Mat4, Vec3, Vec4Swizzles};
use lambert::sample::Lambert;
use shell::{Gpu, Sample};

const WIDTH: u32 = 768;
const HEIGHT: u32 = 576;

const EYE: Vec3 = Vec3::new(0.0, 0.0, 5.0);
const FOV_Y: f32 = std::f32::consts::FRAC_PI_3;

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

/// World point on a face -> frame pixel, via the sample's matrices.
fn project(view_proj: Mat4, point: Vec3) -> (u32, u32) {
    let clip = view_proj * point.extend(1.0);
    let ndc = clip.xyz() / clip.w;
    (
        ((ndc.x + 1.0) * 0.5 * WIDTH as f32).floor() as u32,
        ((1.0 - ndc.y) * 0.5 * HEIGHT as f32).floor() as u32,
    )
}

/// Snapshot 23 in Lambert mode: flat faces have a constant N dot L inside,
/// so each probe equals albedo*(ambient + intensity*d) through the sRGB encode.
#[test]
fn lambert_faces_match_their_dot_products() {
    let Some(ctx) = gpu_context() else {
        println!("skip: no suitable GPU adapter");
        return;
    };
    let gpu = Gpu {
        device: ctx.device.clone(),
        queue: ctx.queue.clone(),
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
    };
    let mut sample = Lambert::init(&gpu).expect("init sample");
    sample.resize(WIDTH, HEIGHT);
    sample.set_show_normals(false);
    let bytes = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });

    let view_proj = glam::camera::rh::proj::directx::perspective(
        FOV_Y,
        WIDTH as f32 / HEIGHT as f32,
        0.1,
        50.0,
    ) * glam::camera::rh::view::look_at_mat4(EYE, Vec3::ZERO, Vec3::Y);
    let expected = |d: f32| {
        let code = expected_code(0.5 * (0.1 + 0.6 * d));
        [code, code, code, 255]
    };

    // Face centers (positions from the CPU table) with their d = cos(tilt).
    let cos30 = (30.0_f32).to_radians().cos();
    let cos45 = (45.0_f32).to_radians().cos();
    let probes = [
        (Vec3::new(-0.95, 0.55, 0.0), cos30, "quad A (+30 deg tilt)"),
        (Vec3::new(0.95, 0.55, 0.0), cos30, "quad B (-30 deg tilt)"),
        (
            Vec3::new(-0.5, -0.8, 0.0),
            cos45,
            "split left (-45 deg normal)",
        ),
        (
            Vec3::new(0.5, -0.8, 0.0),
            1.0,
            "split right (facing the light)",
        ),
        // The crease: probes next to the seam pick different normals.
        (Vec3::new(-0.15, -0.8, 0.0), cos45, "just left of the seam"),
        (Vec3::new(0.15, -0.8, 0.0), 1.0, "just right of the seam"),
    ];
    for (center, d, label) in probes {
        let (px, py) = project(view_proj, center);
        let offset = ((py * WIDTH + px) * 4) as usize;
        let actual: [u8; 4] = bytes[offset..offset + 4].try_into().unwrap();
        let want = expected(d);
        for channel in 0..3 {
            assert!(
                (i32::from(want[channel]) - i32::from(actual[channel])).abs() <= 1,
                "{label} at {center:?}: expected {} got {}",
                want[channel],
                actual[channel]
            );
        }
    }
}
