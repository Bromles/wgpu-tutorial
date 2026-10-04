use foundations_verify::{gpu_context, render_and_readback};
use normal_matrix::sample::NormalMatrix;
use framework::{Gpu, Sample};

const WIDTH: u32 = 768;
const HEIGHT: u32 = 576;

/// The chapter light direction: normalize((1, 0, 1)).
const LIGHT_DIR: glam::Vec3 = glam::Vec3::new(
    std::f32::consts::FRAC_1_SQRT_2,
    0.0,
    std::f32::consts::FRAC_1_SQRT_2,
);

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

/// Face-centre brightness at X scale: world normal is (M^-1)^T * n, dotted
/// with the light.
fn expected_center_code(scale_x: f32) -> u8 {
    let model = glam::Mat4::from_scale(glam::Vec3::new(scale_x, 1.0, 1.0));
    let normal_matrix = glam::Mat3::from_mat4(model).inverse().transpose();
    let local = glam::Vec3::new(1.0, -1.0, 0.0).normalize();
    let world_normal = (normal_matrix * local).normalize();
    let d = world_normal.dot(LIGHT_DIR).max(0.0);
    expected_code(0.5 * (0.1 + 0.6 * d))
}

/// Snapshot 24: with the inverse-transpose normal matrix the face centre
/// follows Lambert at both scales; the two codes must differ.
#[test]
fn normal_matrix_keeps_the_lambert_model_under_scale() {
    let Some(ctx) = gpu_context() else {
        println!("skip: no suitable GPU adapter");
        return;
    };
    let gpu = Gpu {
        device: ctx.device.clone(),
        queue: ctx.queue.clone(),
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
    };
    let mut sample = NormalMatrix::init(&gpu).expect("init sample");
    sample.resize(WIDTH, HEIGHT);

    // Naive M*n differs at scale 2; distinct codes make the probe meaningful.
    let expected_one = expected_center_code(1.0);
    let expected_two = expected_center_code(2.0);
    assert_ne!(
        expected_one, expected_two,
        "the probe must separate the models"
    );

    for (scale_x, expected) in [(1.0, expected_one), (2.0, expected_two)] {
        sample.set_scale(scale_x);
        let bytes = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
            sample.draw(&gpu, encoder, view);
        });
        // Face centre projects to the frame centre; flat face, one normal.
        let actual = pixel(&bytes, WIDTH / 2, HEIGHT / 2);
        assert!(
            (i32::from(expected) - i32::from(actual[0])).abs() <= 1,
            "scale {scale_x}: expected {expected} got {}",
            actual[0]
        );
        assert_eq!(actual[3], 255);
    }
}
