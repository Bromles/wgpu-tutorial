use clip_viewport::sample::{ClipViewport, Interpolation};
use clip_viewport::texture::TEXELS;
use verify::{gpu_context, render_and_readback};
use glam::{Mat4, Vec3, Vec4, Vec4Swizzles};
use framework::{Gpu, Sample};

const WIDTH: u32 = 768;
const HEIGHT: u32 = 576;

fn view_proj() -> Mat4 {
    glam::camera::rh::proj::directx::perspective(
        std::f32::consts::FRAC_PI_3,
        WIDTH as f32 / HEIGHT as f32,
        1.0,
        9.0,
    ) * glam::camera::rh::view::look_at_mat4(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, Vec3::Y)
}

/// Tilted near quad corners in draw order (v = 0 at the edge 2.2 m from the eye).
const TILTED: [[f32; 3]; 4] = [
    [-0.4, 0.2535898, 2.8],
    [0.4, 0.2535898, 2.8],
    [0.4, 0.9464102, 3.2],
    [-0.4, 0.9464102, 3.2],
];

/// Untilted far quad corners, 4 m from the eye.
const FAR_QUAD: [[f32; 3]; 4] = [
    [-0.4, -1.0, 1.0],
    [0.4, -1.0, 1.0],
    [0.4, -0.2, 1.0],
    [-0.4, -0.2, 1.0],
];

/// Corner UVs: (0,0) at the bottom-left, v grows upward.
const QUAD_UVS: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];

fn pixel(bytes: &[u8], px: u32, py: u32) -> [u8; 4] {
    let offset = ((py * WIDTH + px) * 4) as usize;
    bytes[offset..offset + 4].try_into().unwrap()
}

fn pixel_center_ndc(px: u32, py: u32) -> [f32; 2] {
    [
        2.0 * (px as f32 + 0.5) / WIDTH as f32 - 1.0,
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

/// UV at a screen point of a quad: perspective mode interpolates a/w and 1/w
/// then divides; linear mode interpolates raw screen-space values.
fn interpolate_uv(
    world: &[[f32; 3]; 4],
    uvs: &[[f32; 2]; 4],
    view_proj: Mat4,
    ndc: [f32; 2],
    perspective_correct: bool,
) -> [f32; 2] {
    let corners: Vec<([f32; 2], f32, [f32; 2])> = world
        .iter()
        .zip(uvs)
        .map(|(position, uv)| {
            let clip = view_proj * Vec4::new(position[0], position[1], position[2], 1.0);
            let corner_ndc = (clip.xy() / clip.w).to_array();
            (corner_ndc, clip.w, *uv)
        })
        .collect();
    for triangle in [[0usize, 1, 2], [0, 2, 3]] {
        let [ia, ib, ic] = triangle;
        let weights = barycentric(ndc, corners[ia].0, corners[ib].0, corners[ic].0);
        if !weights.iter().all(|w| *w >= 0.0) {
            continue;
        }
        let picked = [corners[ia], corners[ib], corners[ic]];
        if perspective_correct {
            let mut num = [0.0f32; 2];
            let mut den = 0.0f32;
            for (weight, corner) in weights.iter().zip(picked) {
                num[0] += weight * corner.2[0] / corner.1;
                num[1] += weight * corner.2[1] / corner.1;
                den += weight / corner.1;
            }
            return [num[0] / den, num[1] / den];
        }
        let mut uv = [0.0f32; 2];
        for (weight, corner) in weights.iter().zip(picked) {
            uv[0] += weight * corner.2[0];
            uv[1] += weight * corner.2[1];
        }
        return uv;
    }
    panic!("probe lands outside the quad");
}

/// Whole-texel color of a UV, mirroring the shader's textureLoad.
fn texel_color(uv: [f32; 2]) -> [u8; 4] {
    let col = (uv[0].clamp(0.0, 1.0) * 2.0).floor().clamp(0.0, 1.0) as usize;
    let row = (uv[1].clamp(0.0, 1.0) * 2.0).floor().clamp(0.0, 1.0) as usize;
    let offset = (row * 2 + col) * 4;
    [
        TEXELS[offset],
        TEXELS[offset + 1],
        TEXELS[offset + 2],
        TEXELS[offset + 3],
    ]
}

/// Snapshot 19b: probes sit well inside quadrants of both quads; (326, 132)
/// lies between the two seams (NDC y = 0.520 vs 0.555), so they disagree.
#[test]
fn quad_uv_interpolation_matches_both_models() {
    let Some(ctx) = gpu_context() else {
        println!("skip: no suitable GPU adapter");
        return;
    };
    let gpu = Gpu {
        device: ctx.device.clone(),
        queue: ctx.queue.clone(),
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
    };
    let mut sample = ClipViewport::init(&gpu).expect("init sample");
    sample.resize(WIDTH, HEIGHT);
    let view_proj = view_proj();

    let probes: [(u32, u32, &[[f32; 3]; 4]); 5] = [
        (326, 132, &TILTED),
        (437, 172, &TILTED),
        (307, 43, &TILTED),
        (360, 388, &FAR_QUAD),
        (414, 345, &FAR_QUAD),
    ];

    sample.set_interpolation(Interpolation::Perspective);
    let perspective_bytes = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    for (px, py, quad) in probes {
        let uv = interpolate_uv(quad, &QUAD_UVS, view_proj, pixel_center_ndc(px, py), true);
        assert_eq!(
            pixel(&perspective_bytes, px, py),
            texel_color(uv),
            "perspective probe at ({px},{py})"
        );
    }

    sample.set_interpolation(Interpolation::Linear);
    let linear_bytes = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    for (px, py, quad) in probes {
        let uv = interpolate_uv(quad, &QUAD_UVS, view_proj, pixel_center_ndc(px, py), false);
        assert_eq!(
            pixel(&linear_bytes, px, py),
            texel_color(uv),
            "linear probe at ({px},{py})"
        );
    }

    // Between the two seams the same pixel shows different quadrants.
    assert_ne!(
        pixel(&perspective_bytes, 326, 132),
        pixel(&linear_bytes, 326, 132),
        "the seam probe should expose the interpolation difference"
    );
}
