use verify::{gpu_context, render_and_readback};
use glam::{Vec3, Vec4Swizzles};
use shadow_mapping::params::SceneParams;
use shadow_mapping::sample::ShadowMapping;
use shadow_mapping::scene;
use framework::{Gpu, Sample};

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

/// Analytic shadow test, GPU-independent: does the ray from a floor point
/// along L hit the cube? Slab test against the box AABB.
fn in_cube_shadow(point: Vec3, cube_offset_x: f32) -> bool {
    let l = scene::light_dir();
    let min = Vec3::new(cube_offset_x - 0.5, 0.01, -0.5);
    let max = Vec3::new(cube_offset_x + 0.5, 1.01, 0.5);
    let (mut t_min, mut t_max) = (0.0f32, f32::INFINITY);
    for axis in 0..3 {
        let (origin, direction, lo, hi) = (point[axis], l[axis], min[axis], max[axis]);
        if direction.abs() < 1e-9 {
            if origin < lo || origin > hi {
                return false;
            }
        } else {
            let (mut near_t, mut far_t) = ((lo - origin) / direction, (hi - origin) / direction);
            if near_t > far_t {
                std::mem::swap(&mut near_t, &mut far_t);
            }
            t_min = t_min.max(near_t);
            t_max = t_max.min(far_t);
            if t_min > t_max {
                return false;
            }
        }
    }
    true
}

/// Probes must sit inside the light frustum so the comparison path runs.
fn assert_inside_light_frustum(point: Vec3) {
    let clip = scene::light_view_proj() * point.extend(1.0);
    let ndc = clip.xyz() / clip.w;
    assert!(
        ndc.x > -1.0 && ndc.x < 1.0 && ndc.y > -1.0 && ndc.y < 1.0 && ndc.z > 0.0 && ndc.z < 1.0,
        "probe {point:?} must sit inside the light frustum, ndc {ndc:?}"
    );
}

/// Segment-slab test against the cube AABB over the span (0, t_max).
fn sightline_hits_cube(origin: Vec3, dir: Vec3, t_max: f32, cube_offset_x: f32) -> bool {
    let min = Vec3::new(cube_offset_x - 0.5, 0.01, -0.5);
    let max = Vec3::new(cube_offset_x + 0.5, 1.01, 0.5);
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

/// Analytic screen masks per pixel centre: `floor_visible` = ray hits the
/// floor quad unobstructed; `shadow` = point in the cast shadow.
fn visible_shadow_masks(cube_offset_x: f32) -> (Vec<bool>, Vec<bool>) {
    let inv = scene::camera_view_proj().inverse();
    let unproject = |px: u32, py: u32, ndc_z: f32| -> Vec3 {
        let ndc_x = (px as f32 + 0.5) / WIDTH as f32 * 2.0 - 1.0;
        let ndc_y = 1.0 - (py as f32 + 0.5) / HEIGHT as f32 * 2.0;
        let clip = inv * glam::Vec4::new(ndc_x, ndc_y, ndc_z, 1.0);
        clip.truncate() / clip.w
    };
    let total = (WIDTH * HEIGHT) as usize;
    let mut floor_visible = vec![false; total];
    let mut shadow = vec![false; total];
    for py in 0..HEIGHT {
        for px in 0..WIDTH {
            let near = unproject(px, py, 0.0);
            let far = unproject(px, py, 1.0);
            let dir = far - near;
            if dir.y >= 0.0 {
                continue;
            }
            let t = -near.y / dir.y;
            let point = near + dir * t;
            if point.x.abs() > 2.0 || point.z.abs() > 2.0 {
                continue;
            }
            if sightline_hits_cube(near, dir, t - 1e-4, cube_offset_x) {
                continue;
            }
            let index = (py * WIDTH + px) as usize;
            floor_visible[index] = true;
            shadow[index] = in_cube_shadow(point, cube_offset_x);
        }
    }
    (shadow, floor_visible)
}

/// Two-sided containment with small dilation: masks may disagree near
/// boundaries (texel rasterization, hover-gap bleed); further out = a bug.
fn assert_masks_agree(actual: &[bool], expected: &[bool], label: &str) {
    let at = |mask: &[bool], x: i64, y: i64, window: i64| {
        (0..=window)
            .flat_map(|dx| (0..=window).map(move |dy| (dx, dy)))
            .any(|(dx, dy)| {
                let (nx, ny) = (x + dx, y + dy);
                nx >= 0
                    && ny >= 0
                    && (nx as u32) < WIDTH
                    && (ny as u32) < HEIGHT
                    && mask[(ny as u32 * WIDTH + nx as u32) as usize]
            })
    };
    let mut offenders_a = Vec::new();
    let mut offenders_b = Vec::new();
    for py in 0..HEIGHT as i64 {
        for px in 0..WIDTH as i64 {
            let index = (py as u32 * WIDTH + px as u32) as usize;
            if actual[index] && !at(expected, px - 4, py - 4, 8) {
                offenders_a.push((px, py));
            }
            if expected[index] && !at(actual, px - 2, py - 2, 4) {
                offenders_b.push((px, py));
            }
        }
    }
    println!(
        "{label}: {}/{} rendered-outside-analytic, {}/{} analytic-not-rendered",
        offenders_a.len(),
        actual.iter().filter(|&&v| v).count(),
        offenders_b.len(),
        expected.iter().filter(|&&v| v).count()
    );
    if !offenders_a.is_empty() {
        println!(
            "  first offenders: {:?}",
            &offenders_a[..offenders_a.len().min(20)]
        );
    }
    assert!(
        offenders_a.len() <= 4,
        "{label}: rendered shadow spread beyond the analytic footprint: \
         {offenders_a:?} (texel bleed is allowed only for a couple of \
         gap-edge pixels, not for whole regions)"
    );
    assert!(
        offenders_b.is_empty(),
        "{label}: analytic shadow did not render: {offenders_b:?}"
    );
}

/// Snapshot 31a: the shadow map splits the floor into Lambert and
/// ambient-only; H removes only the shadow term; M moves shadow and cube.
#[test]
fn shadow_map_splits_lambert_on_the_floor() {
    let Some(ctx) = gpu_context() else {
        println!("skip: no suitable GPU adapter");
        return;
    };
    let gpu = Gpu {
        device: ctx.device.clone(),
        queue: ctx.queue.clone(),
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
    };
    let mut sample = ShadowMapping::init(&gpu).expect("init sample");
    sample.resize(WIDTH, HEIGHT);

    // Probes: a far lit floor point and the analytic shadow center.
    let lit_probe = Vec3::new(1.2, 0.0, 1.2);
    let shadow_probe = scene::cube_shadow_center(0.0);
    assert!(
        !in_cube_shadow(lit_probe, 0.0),
        "lit probe must miss the cube"
    );
    assert!(
        in_cube_shadow(shadow_probe, 0.0),
        "shadow probe must hit the cube"
    );
    assert_inside_light_frustum(lit_probe);
    assert_inside_light_frustum(shadow_probe);

    let params = SceneParams::chapter();
    let lit = expected_color(params.shade(Vec3::Y, 1.0));
    let blocked = expected_color(params.shade(Vec3::Y, 0.0));

    // (a) Outside the shadow: the pure Lambert value of the floor.
    let bytes_with_shadows = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    let (px, py) = covering_pixel(lit_probe);
    let lit_pixel = pixel(&bytes_with_shadows, px, py);
    assert_color(lit_pixel, lit, "lit floor probe");

    // H-toggle diff: changed pixels keep ambient-only and match the analytic footprint.
    sample.set_shadows(false);
    let bytes_without = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    let (expected_shadow, floor_visible) = visible_shadow_masks(0.0);
    let mut changed = 0usize;
    let mut floor_shadow = 0usize;
    let mut changed_mask = vec![false; (WIDTH * HEIGHT) as usize];
    for offset in (0..bytes_with_shadows.len()).step_by(4) {
        let index = offset / 4;
        let on = &bytes_with_shadows[offset..offset + 3];
        let off = &bytes_without[offset..offset + 3];
        if on == off {
            continue;
        }
        changed += 1;
        assert_color(
            [on[0], on[1], on[2], 255],
            blocked,
            "every H-changed pixel keeps only ambient",
        );
        // Visible floor pixel: before the toggle it must be exactly lit.
        if floor_visible[index] {
            assert_color([off[0], off[1], off[2], 255], lit, "floor pixel before H");
            floor_shadow += 1;
            changed_mask[index] = true;
        }
    }
    assert!(
        floor_shadow > 20,
        "the visible floor shadow must cover dozens of pixels, found {floor_shadow} (changed total {changed})"
    );
    assert_masks_agree(&changed_mask, &expected_shadow, "rest pose");
    let (px, py) = covering_pixel(lit_probe);
    assert_color(pixel(&bytes_without, px, py), lit, "lit probe with H off");
}

/// Moving the caster one world unit moves the shadow: the H-diff of the
/// moved pose is ambient-only exactly where the moved cube casts it.
#[test]
fn moving_the_cube_moves_the_shadow() {
    let Some(ctx) = gpu_context() else {
        println!("skip: no suitable GPU adapter");
        return;
    };
    let gpu = Gpu {
        device: ctx.device.clone(),
        queue: ctx.queue.clone(),
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
    };
    let mut sample = ShadowMapping::init(&gpu).expect("init sample");
    sample.resize(WIDTH, HEIGHT);
    sample.set_cube_offset(1.0);

    let old_center = scene::cube_shadow_center(0.0);
    let new_center = scene::cube_shadow_center(1.0);
    assert!(!in_cube_shadow(old_center, 1.0));
    assert!(in_cube_shadow(new_center, 1.0));
    assert_inside_light_frustum(new_center);

    let params = SceneParams::chapter();
    let lit = expected_color(params.shade(Vec3::Y, 1.0));
    let blocked = expected_color(params.shade(Vec3::Y, 0.0));

    let bytes_moved = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    sample.set_shadows(false);
    let bytes_moved_no_shadow = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });

    let (expected_shadow, floor_visible) = visible_shadow_masks(1.0);
    let mut floor_shadow = 0usize;
    let mut changed_mask = vec![false; (WIDTH * HEIGHT) as usize];
    for offset in (0..bytes_moved.len()).step_by(4) {
        let index = offset / 4;
        let on = &bytes_moved[offset..offset + 3];
        let off = &bytes_moved_no_shadow[offset..offset + 3];
        if on == off {
            continue;
        }
        assert_color(
            [on[0], on[1], on[2], 255],
            blocked,
            "moved pose: every changed pixel keeps only ambient",
        );
        if floor_visible[index] {
            assert_color(
                [off[0], off[1], off[2], 255],
                lit,
                "moved pose: floor pixel before H",
            );
            floor_shadow += 1;
            changed_mask[index] = true;
        }
    }
    assert!(
        floor_shadow > 20,
        "the moved cube must cast a visible floor shadow: {floor_shadow} pixels"
    );
    // The shadow must land where the moved cube casts it (offset 1.0).
    assert_masks_agree(&changed_mask, &expected_shadow, "moved pose");
}
