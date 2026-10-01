use foundations_verify::{ComparisonType, gpu_context, reference_path, render_and_readback};
use shell::{Gpu, Sample};
use vertex_colors::sample::VertexColors;

const WIDTH: u32 = 768;
const HEIGHT: u32 = 576;
const A: [f32; 2] = [-0.75, -0.75];
const B: [f32; 2] = [0.75, -0.75];
const C: [f32; 2] = [0.0, 0.75];

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

/// Barycentric weights of a pixel center against the chapter triangle; the
/// vertex colors are red/green/blue, so the weights are the linear RGB.
fn barycentric_weights(p: [f32; 2]) -> [f32; 3] {
    let cross = |u: [f32; 2], v: [f32; 2]| u[0] * v[1] - u[1] * v[0];
    let sub =
        |q: [f32; 2], r: [f32; 2]| cross([q[0] - p[0], q[1] - p[1]], [r[0] - p[0], r[1] - p[1]]);
    let total = cross([B[0] - A[0], B[1] - A[1]], [C[0] - A[0], C[1] - A[1]]);
    [sub(B, C) / total, sub(C, A) / total, sub(A, B) / total]
}

fn pixel_center_ndc(px: u32, py: u32) -> [f32; 2] {
    [
        (px as f32 + 0.5) / WIDTH as f32 * 2.0 - 1.0,
        1.0 - 2.0 * (py as f32 + 0.5) / HEIGHT as f32,
    ]
}

fn pixel(bytes: &[u8], px: u32, py: u32) -> [u8; 4] {
    let offset = ((py * WIDTH + px) * 4) as usize;
    bytes[offset..offset + 4].try_into().unwrap()
}

/// Snapshot 06: per-vertex colors; interior pixels must match the barycentric
/// model, then the whole frame is compared against the reference.
#[test]
fn vertex_colors_match_barycentric_model_and_reference() {
    let Some(ctx) = gpu_context() else {
        println!("skip: no suitable GPU adapter");
        return;
    };
    let gpu = Gpu {
        device: ctx.device.clone(),
        queue: ctx.queue.clone(),
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
    };
    let mut sample = VertexColors::init(&gpu).expect("init sample");
    let bytes = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });

    for (px, py) in [(384, 300), (200, 400), (420, 180)] {
        let weights = barycentric_weights(pixel_center_ndc(px, py));
        let expected = [
            expected_code(weights[0]),
            expected_code(weights[1]),
            expected_code(weights[2]),
        ];
        let actual = pixel(&bytes, px, py);
        for channel in 0..3 {
            assert!(
                (i32::from(expected[channel]) - i32::from(actual[channel])).abs() <= 1,
                "pixel ({px},{py}) channel {channel}: expected {} got {}",
                expected[channel],
                actual[channel]
            );
        }
    }

    let path = reference_path("vertex-colors.png");
    foundations_verify::compare_reference(
        &path,
        &ctx.adapter_info,
        WIDTH,
        HEIGHT,
        &bytes,
        &[
            ComparisonType::Mean(0.02),
            ComparisonType::Percentile {
                percentile: 0.99,
                threshold: 0.35,
            },
        ],
    );
}
