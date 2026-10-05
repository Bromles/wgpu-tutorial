use wgpu::TextureFormat;
use glam::{Mat4, Vec3};
use std::f32::consts::FRAC_PI_2;

use verify::{gpu_context, render_and_readback};
use matrix_compose::sample::MatrixCompose;
use framework::{Gpu, Sample};

const WIDTH: u32 = 768;
const HEIGHT: u32 = 576;

const VERTICES: [[f32; 2]; 3] = [[-0.25, -0.2], [0.35, -0.15], [0.0, 0.3]];
const COLORS: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

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

fn barycentric(p: [f32; 2],
a: [f32; 2],
b: [f32; 2],
c: [f32; 2]) -> [f32; 3] {
    let cross = |u: [f32; 2], v: [f32; 2]| u[0] * v[1] - u[1] * v[0];
    let total = cross([b[0] - a[0], b[1] - a[1]], [c[0] - a[0], c[1] - a[1]]);
    let sub =
        |q: [f32; 2],
        r: [f32; 2]| cross([q[0] - p[0], q[1] - p[1]], [r[0] - p[0], r[1] - p[1]]);
    [sub(b, c) / total, sub(c, a) / total, sub(a, b) / total]
}

/// Snapshot 16: T*R and R*T place the triangle differently; each frame is
/// checked against the CPU composition of the same matrix.
#[test]
fn matrix_compose_order_matters() {
    let Some(ctx) = gpu_context() else {
        println!("skip: no suitable GPU adapter");
        return;
    };
    let gpu = Gpu {
        device: ctx.device.clone(),
        queue: ctx.queue.clone(),
        format: TextureFormat::Rgba8UnormSrgb,
    };
    let mut sample = MatrixCompose::init(&gpu).expect("init sample");
    let translate = Mat4::from_translation(Vec3::new(0.25, 0.0, 0.0));
    let rotate = Mat4::from_rotation_z(FRAC_PI_2);
    for transform in [translate * rotate, rotate * translate] {
        sample.set_transform(transform);
        let bytes = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
            sample.draw(&gpu, encoder, view);
        });
        let placed: Vec<[f32; 2]> = VERTICES
            .iter()
            .map(|v| {
                let world = transform.transform_point3(Vec3::new(v[0], v[1], 0.5));
                [world.x, world.y]
            })
            .collect();
        let mut checked = 0usize;
        for py in (100..500).step_by(7) {
            for px in (100..680).step_by(7) {
                let ndc = [
                    (px as f32 + 0.5) / WIDTH as f32 * 2.0 - 1.0,
                    1.0 - 2.0 * (py as f32 + 0.5) / HEIGHT as f32,
                ];
                let weights = barycentric(ndc, placed[0], placed[1], placed[2]);
                if !weights.iter().all(|w| *w > 0.02 && *w < 0.98) {
                    continue;
                }
                let expected = [
                    expected_code(
                        weights[0] * COLORS[0][0]
                            + weights[1] * COLORS[1][0]
                            + weights[2] * COLORS[2][0],
                    ),
                    expected_code(
                        weights[0] * COLORS[0][1]
                            + weights[1] * COLORS[1][1]
                            + weights[2] * COLORS[2][1],
                    ),
                    expected_code(
                        weights[0] * COLORS[0][2]
                            + weights[1] * COLORS[1][2]
                            + weights[2] * COLORS[2][2],
                    ),
                ];
                let offset = ((py * WIDTH + px) * 4) as usize;
                let actual: [u8; 4] = bytes[offset..offset + 4].try_into().unwrap();
                for channel in 0..3 {
                    assert!(
                        (i32::from(expected[channel]) - i32::from(actual[channel])).abs() <= 1,
                        "pixel ({px},{py}) channel {channel}: expected {} got {}",
                        expected[channel],
                        actual[channel]
                    );
                }
                checked += 1;
            }
        }
        assert!(checked >= 50, "expected interior probes, got {checked}");
    }
}
