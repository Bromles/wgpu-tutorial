use wgpu::TextureFormat;
use cube_uv::mesh::{FACE_VIEWS, VERTICES};
use cube_uv::sample::CubeUv;
use framework::{Gpu, Sample};
use glam::{Mat4, Vec3, Vec4Swizzles};
use verify::{gpu_context, render_and_readback};

const WIDTH: u32 = 768;

use glam::camera::rh::proj::directx::orthographic;
use glam::camera::rh::view::look_at_mat4;
const HEIGHT: u32 = 576;

/// sRGB codes of the four texture quadrants and the origin marker.
const MARKER: [u8; 4] = [0, 0, 0, 255];
const RED: [u8; 4] = [255, 0, 0, 255];
const GREEN: [u8; 4] = [0, 255, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];
const YELLOW: [u8; 4] = [255, 255, 0, 255];

/// The snapshot's fixed 4x3 orthographic projection matching the 4:3 frame.
fn ortho() -> Mat4 {
    orthographic(-2.0, 2.0, -1.5, 1.5, 0.1, 10.0)
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

fn pixel(bytes: &[u8], px: u32, py: u32) -> [u8; 4] {
    let offset = ((py * WIDTH + px) * 4) as usize;
    bytes[offset..offset + 4].try_into().unwrap()
}

fn assert_texel(bytes: &[u8], view_proj: Mat4, world: Vec3, expected: [u8; 4], label: &str) {
    let (px, py) = project(view_proj, world);
    assert_eq!(
        pixel(bytes, px, py),
        expected,
        "{label} at world {world:?} pixel ({px},{py})"
    );
}

/// Snapshot 22: each face in its fixed view shows the marked quadrant
/// layout of the CPU table — positions and UVs travel together.
#[test]
fn cube_faces_show_their_quadrant_layout() {
    let Some(ctx) = gpu_context() else {
        println!("skip: no suitable GPU adapter");
        return;
    };
    let gpu = Gpu {
        device: ctx.device.clone(),
        queue: ctx.queue.clone(),
        format: TextureFormat::Rgba8UnormSrgb,
    };
    let mut sample = CubeUv::init(&gpu).expect("init sample");
    sample.resize(WIDTH, HEIGHT);

    // Probes sit inside quadrants away from seams and the marker.
    let probes = [
        ((0.375, 0.375), RED),
        ((0.625, 0.375), GREEN),
        ((0.375, 0.625), BLUE),
        ((0.625, 0.625), YELLOW),
        ((0.1, 0.1), MARKER),
    ];

    // UV -> world mapping derived from the vertex table - orientation-honest.
    let world_on_face = |face: usize, u: f32, v: f32| -> Vec3 {
        let corners = &VERTICES[face * 4..face * 4 + 4];
        let pick = |want: [f32; 2]| {
            corners
                .iter()
                .find(|c| c.uv == want)
                .unwrap_or_else(|| panic!("face {face} lacks uv {want:?}"))
                .position
        };
        let p00 = pick([0.0, 0.0]);
        let p10 = pick([1.0, 0.0]);
        let p01 = pick([0.0, 1.0]);
        let p11 = pick([1.0, 1.0]);
        let w = |a: [f32; 4], t: f32| Vec3::from_slice(&a[..3]) * t;
        w(p00, (1.0 - u) * (1.0 - v))
            + w(p10, u * (1.0 - v))
            + w(p01, (1.0 - u) * v)
            + w(p11, u * v)
    };

    // Every face in its fixed view (keys 1..6) shows the same quadrant layout.
    for (face, label) in [
        (0usize, "+Z"),
        (1, "-Z"),
        (2, "+X"),
        (3, "-X"),
        (4, "+Y"),
        (5, "-Y"),
    ] {
        sample.set_face(face);
        let bytes = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
            sample.draw(&gpu, encoder, view);
        });
        let view_proj =
            ortho() * look_at_mat4(FACE_VIEWS[face].eye, Vec3::ZERO, FACE_VIEWS[face].up);
        for ((u, v), expected) in probes {
            let world = world_on_face(face, u, v);
            assert_texel(&bytes, view_proj, world, expected, &format!("{label} face"));
        }
    }
}
