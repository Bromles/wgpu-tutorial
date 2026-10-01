use foundations_verify::{gpu_context, render_and_readback};
use glam::{Vec3, Vec4Swizzles};
use shadow_pcf::params::SceneParams;
use shadow_pcf::sample::ShadowPcf;
use shadow_pcf::scene;
use shell::{Gpu, Sample};

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

fn expected_color(linear: Vec3) -> [u8; 4] {
    [
        expected_code(linear.x),
        expected_code(linear.y),
        expected_code(linear.z),
        255,
    ]
}

fn pixel(bytes: &[u8], px: u32, py: u32) -> [u8; 4] {
    let offset = ((py * WIDTH + px) * 4) as usize;
    bytes[offset..offset + 4].try_into().unwrap()
}

/// World point -> the pixel covering it, via the sample's camera chain.
fn covering_pixel(point: Vec3) -> (u32, u32) {
    let clip = scene::camera_view_proj() * point.extend(1.0);
    let ndc = clip.xyz() / clip.w;
    let px = ((ndc.x * 0.5 + 0.5) * WIDTH as f32 - 0.5).round() as u32;
    let py = ((0.5 - ndc.y * 0.5) * HEIGHT as f32 - 0.5).round() as u32;
    (px.min(WIDTH - 1), py.min(HEIGHT - 1))
}

/// Probes one code away from the hardware sRGB encode.
fn assert_color(actual: [u8; 4], expected: [u8; 4], label: &str) {
    for channel in 0..3 {
        assert!(
            (i32::from(expected[channel]) - i32::from(actual[channel])).abs() <= 1,
            "{label}: expected {} got {}",
            expected[channel],
            actual[channel]
        );
    }
}

/// The chapter 31a control probe, unchanged by the move to this crate.
const LIT_PROBE: Vec3 = Vec3::new(1.2, 0.0, 1.2);

fn lit_color() -> [u8; 4] {
    expected_color(SceneParams::chapter().shade(Vec3::Y, 1.0))
}

// Reserved for pinned probes: the analytic blocked-pixel color.
fn blocked_color() -> [u8; 4] {
    expected_color(SceneParams::chapter().shade(Vec3::Y, 0.0))
}

/// All 3x3 PCF visibility values: k of nine taps open -> albedo * (ambient
/// + intensity * d * k/9), k = 0..=9 - results are averaged, never depths.
fn pcf_colors() -> Vec<[u8; 4]> {
    let params = SceneParams::chapter();
    (0..=9)
        .map(|k| expected_color(params.shade(Vec3::Y, k as f32 / 9.0)))
        .collect()
}

/// Segment-slab test against the cube AABB over the span (0, t_max).
fn sightline_hits_cube(origin: Vec3, dir: Vec3, t_max: f32) -> bool {
    let min = Vec3::new(-0.5, 0.01, -0.5);
    let max = Vec3::new(0.5, 1.01, 0.5);
    let (mut t_enter, mut t_exit) = (0.0f32, t_max);
    for axis in 0..3 {
        if dir[axis].abs() < 1e-9 {
            if origin[axis] < min[axis] || origin[axis] > max[axis] {
                return false;
            }
        } else {
            let (mut near_t, mut far_t) = (
                (min[axis] - origin[axis]) / dir[axis],
                (max[axis] - origin[axis]) / dir[axis],
            );
            if near_t > far_t {
                std::mem::swap(&mut near_t, &mut far_t);
            }
            t_enter = t_enter.max(near_t);
            t_exit = t_exit.min(far_t);
            if t_enter > t_exit {
                return false;
            }
        }
    }
    t_enter < t_exit
}

/// Screen mask of pixels whose ray lands on the floor quad with a clear
/// sightline (geometric identification; the cube shares the floor albedo).
fn floor_visible_mask() -> Vec<bool> {
    let inv = scene::camera_view_proj().inverse();
    let unproject = |px: u32, py: u32, ndc_z: f32| -> Vec3 {
        let ndc_x = (px as f32 + 0.5) / WIDTH as f32 * 2.0 - 1.0;
        let ndc_y = 1.0 - (py as f32 + 0.5) / HEIGHT as f32 * 2.0;
        let clip = inv * glam::Vec4::new(ndc_x, ndc_y, ndc_z, 1.0);
        clip.truncate() / clip.w
    };
    let mut mask = vec![false; (WIDTH * HEIGHT) as usize];
    for py in 0..HEIGHT {
        for px in 0..WIDTH {
            let near = unproject(px, py, 0.0);
            let dir = unproject(px, py, 1.0) - near;
            if dir.y >= 0.0 {
                continue;
            }
            let t = -near.y / dir.y;
            let point = near + dir * t;
            if point.x.abs() > 2.0 || point.z.abs() > 2.0 {
                continue;
            }
            if sightline_hits_cube(near, dir, t - 1e-4) {
                continue;
            }
            mask[(py * WIDTH + px) as usize] = true;
        }
    }
    mask
}

/// Pass clear color; the top-left corner is sky above every surface.
fn background_color() -> [u8; 4] {
    expected_color(Vec3::new(0.1, 0.1, 0.12))
}

/// Snapshot 31b: a texel-scale bias (0.0005) keeps both control points -
/// the shadow interior is far deeper than the bias.
#[test]
fn small_bias_keeps_the_control_points() {
    let Some(ctx) = gpu_context() else {
        println!("skip: no suitable GPU adapter");
        return;
    };
    let gpu = Gpu {
        device: ctx.device.clone(),
        queue: ctx.queue.clone(),
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
    };
    let mut sample = ShadowPcf::init(&gpu).expect("init sample");
    sample.resize(WIDTH, HEIGHT);
    sample.set_bias(0.0005);

    let bytes = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    let (px, py) = covering_pixel(LIT_PROBE);
    assert_color(
        pixel(&bytes, px, py),
        lit_color(),
        "lit floor probe, bias 0.0005",
    );

    sample.set_shadows(false);
    let bytes_without = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    let floor_visible = floor_visible_mask();
    let mut blocked_floor = 0usize;
    for offset in (0..bytes.len()).step_by(4) {
        let index = offset / 4;
        if !floor_visible[index] {
            continue;
        }
        let on = &bytes[offset..offset + 3];
        let off = &bytes_without[offset..offset + 3];
        if on == off {
            continue;
        }
        assert_color(
            [off[0], off[1], off[2], 255],
            lit_color(),
            "floor pixel before H",
        );
        assert_color(
            [on[0], on[1], on[2], 255],
            blocked_color(),
            "floor pixel after H",
        );
        blocked_floor += 1;
    }
    assert!(
        blocked_floor > 20,
        "the visible shadow must cover dozens of floor pixels, found {blocked_floor}"
    );
}

/// PCF averages comparison RESULTS: visibility is k/9. Interior pixels
/// hold ambient-only, edges one of the ten k/9 colours, lit floor Lambert.
#[test]
fn pcf_softens_the_edge_but_keeps_the_control_points() {
    let Some(ctx) = gpu_context() else {
        println!("skip: no suitable GPU adapter");
        return;
    };
    let gpu = Gpu {
        device: ctx.device.clone(),
        queue: ctx.queue.clone(),
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
    };
    let mut sample = ShadowPcf::init(&gpu).expect("init sample");
    sample.resize(WIDTH, HEIGHT);
    sample.set_bias(0.0005);
    sample.set_pcf(true);

    let bytes = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    let (px, py) = covering_pixel(LIT_PROBE);
    assert_color(
        pixel(&bytes, px, py),
        lit_color(),
        "lit floor probe under PCF",
    );
    assert_color(
        pixel(&bytes, 0, 0),
        background_color(),
        "background corner under PCF",
    );

    // Interior (all taps blocked) must hold ambient-only; edges one of the k/9 colours.
    sample.set_shadows(false);
    let bytes_without = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    let floor_visible = floor_visible_mask();
    let mut blocked_floor = 0usize;
    let mut darkened = 0usize;
    for offset in (0..bytes.len()).step_by(4) {
        let index = offset / 4;
        if !floor_visible[index] {
            continue;
        }
        let on = &bytes[offset..offset + 3];
        let off = &bytes_without[offset..offset + 3];
        if on == off {
            continue;
        }
        assert_color(
            [off[0], off[1], off[2], 255],
            lit_color(),
            "PCF: floor pixel before H",
        );
        darkened += 1;
        let matches_some = pcf_colors()
            .iter()
            .any(|color| (0..3).all(|c| (i32::from(on[c]) - i32::from(color[c])).abs() <= 1));
        assert!(
            matches_some,
            "PCF floor pixel must be one of the ten k/9 colours, got {:?}",
            [on[0], on[1], on[2]]
        );
        let is_blocked =
            (0..3).all(|c| (i32::from(on[c]) - i32::from(blocked_color()[c])).abs() <= 1);
        if is_blocked {
            blocked_floor += 1;
        }
    }
    assert!(
        darkened > 20,
        "PCF must darken dozens of floor pixels, found {darkened}"
    );
    assert!(
        blocked_floor > 5,
        "the shadow interior (all nine taps blocked) must exist under PCF: {blocked_floor}"
    );
    // The softened edge is real: not every darkened pixel is fully blocked.
    assert!(
        blocked_floor < darkened,
        "PCF must produce partially lit edge pixels, not a hard boundary"
    );
}

/// The R key switches map and bind group together. At 512 the texel doubles
/// but the control points survive: lit exact, darkened = ambient-only.
#[test]
fn half_resolution_map_keeps_the_control_points() {
    let Some(ctx) = gpu_context() else {
        println!("skip: no suitable GPU adapter");
        return;
    };
    let gpu = Gpu {
        device: ctx.device.clone(),
        queue: ctx.queue.clone(),
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
    };
    let mut sample = ShadowPcf::init(&gpu).expect("init sample");
    sample.resize(WIDTH, HEIGHT);
    sample.set_bias(0.0005);
    sample.set_map_size(512);

    let bytes = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    let (px, py) = covering_pixel(LIT_PROBE);
    assert_color(pixel(&bytes, px, py), lit_color(), "lit floor probe at 512");

    sample.set_shadows(false);
    let bytes_without = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    let floor_visible = floor_visible_mask();
    let mut blocked_floor = 0usize;
    for offset in (0..bytes.len()).step_by(4) {
        let index = offset / 4;
        if !floor_visible[index] {
            continue;
        }
        let on = &bytes[offset..offset + 3];
        let off = &bytes_without[offset..offset + 3];
        if on == off {
            continue;
        }
        assert_color(
            [off[0], off[1], off[2], 255],
            lit_color(),
            "512: floor pixel before H",
        );
        assert_color(
            [on[0], on[1], on[2], 255],
            blocked_color(),
            "512: floor pixel after H",
        );
        blocked_floor += 1;
    }
    assert!(
        blocked_floor > 20,
        "512 map must still shadow dozens of floor pixels: {blocked_floor}"
    );
}
