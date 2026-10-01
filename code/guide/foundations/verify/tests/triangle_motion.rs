use foundations_verify::{gpu_context, render_and_readback};
use shell::{Gpu, Sample};
use triangle_motion::params::Params;
use triangle_motion::sample::TriangleMotion;

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

fn transform(p: [f32; 2], params: &Params) -> [f32; 2] {
    let s = params.scale_angle.x;
    let (sin, cos) = params.scale_angle.y.sin_cos();
    let local = [p[0] * s, p[1] * s];
    [
        cos * local[0] - sin * local[1] + params.translate.x,
        sin * local[0] + cos * local[1] + params.translate.y,
    ]
}

fn pixel_center_ndc(px: u32, py: u32) -> [f32; 2] {
    [
        (px as f32 + 0.5) / WIDTH as f32 * 2.0 - 1.0,
        1.0 - 2.0 * (py as f32 + 0.5) / HEIGHT as f32,
    ]
}

fn barycentric(p: [f32; 2], a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> [f32; 3] {
    let cross = |u: [f32; 2], v: [f32; 2]| u[0] * v[1] - u[1] * v[0];
    let total = cross([b[0] - a[0], b[1] - a[1]], [c[0] - a[0], c[1] - a[1]]);
    let sub =
        |q: [f32; 2], r: [f32; 2]| cross([q[0] - p[0], q[1] - p[1]], [r[0] - p[0], r[1] - p[1]]);
    [sub(b, c) / total, sub(c, a) / total, sub(a, b) / total]
}

fn pixel(bytes: &[u8], px: u32, py: u32) -> [u8; 4] {
    let offset = ((py * WIDTH + px) * 4) as usize;
    bytes[offset..offset + 4].try_into().unwrap()
}

/// Snapshot 15: with explicit (translate, scale, angle) the frame must match
/// the point model: scale, then rotate, then translate.
#[test]
fn triangle_motion_matches_point_model() {
    let Some(ctx) = gpu_context() else {
        println!("skip: no suitable GPU adapter");
        return;
    };
    let gpu = Gpu {
        device: ctx.device.clone(),
        queue: ctx.queue.clone(),
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
    };
    let mut sample = TriangleMotion::init(&gpu).expect("init sample");

    for params in [
        Params::new(glam::Vec2::new(0.0, 0.0), glam::Vec2::new(1.0, 0.0)),
        Params::new(
            glam::Vec2::new(0.1, 0.2),
            glam::Vec2::new(1.0, std::f32::consts::FRAC_PI_2),
        ),
        Params::new(glam::Vec2::new(-0.2, 0.1), glam::Vec2::new(0.5, 0.7)),
    ] {
        sample.set_params(params);
        let bytes = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
            sample.draw(&gpu, encoder, view);
        });
        let placed: Vec<[f32; 2]> = VERTICES.iter().map(|v| transform(*v, &params)).collect();
        let mut checked = 0usize;
        for py in (100..500).step_by(7) {
            for px in (100..680).step_by(7) {
                let weights =
                    barycentric(pixel_center_ndc(px, py), placed[0], placed[1], placed[2]);
                // Strictly interior pixels only: away from rasterized edges.
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
                let actual = pixel(&bytes, px, py);
                for channel in 0..3 {
                    assert!(
                        (i32::from(expected[channel]) - i32::from(actual[channel])).abs() <= 1,
                        "params {params:?} pixel ({px},{py}) channel {channel}: expected {} got {}",
                        expected[channel],
                        actual[channel]
                    );
                }
                checked += 1;
            }
        }
        assert!(
            checked >= 50,
            "expected enough interior probes, got {checked}"
        );
    }
}
