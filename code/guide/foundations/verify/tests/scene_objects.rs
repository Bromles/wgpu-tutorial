use foundations_verify::{gpu_context, gpu_context_with, render_and_readback};
use glam::{Vec3, Vec4};
use scene_objects::camera;
use scene_objects::sample::SceneObjects;
use scene_objects::scene::{self, Material};
use framework::{Gpu, Sample};

const WIDTH: u32 = 768;
const HEIGHT: u32 = 576;
const ASPECT: f32 = WIDTH as f32 / HEIGHT as f32;

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

/// World point of the z = 0 plane under a pixel centre: invert the exact
/// P*V chain (all plane points share one NDC depth, provided by the origin).
fn plane_point(px: u32, py: u32) -> Vec3 {
    let view_proj = camera::view_proj(ASPECT);
    let origin = view_proj * Vec4::new(0.0, 0.0, 0.0, 1.0);
    let ndc_z = origin.z / origin.w;
    let ndc_x = (px as f32 + 0.5) / WIDTH as f32 * 2.0 - 1.0;
    let ndc_y = 1.0 - 2.0 * (py as f32 + 0.5) / HEIGHT as f32;
    let world = view_proj.inverse() * Vec4::new(ndc_x, ndc_y, ndc_z, 1.0);
    world.truncate() / world.w
}

/// Pixel covering a world point of the plane.
fn covering_pixel(x: f32, y: f32) -> (u32, u32) {
    let view_proj = camera::view_proj(ASPECT);
    let clip = view_proj * Vec4::new(x, y, 0.0, 1.0);
    let ndc = clip.truncate() / clip.w;
    let px = ((ndc.x * 0.5 + 0.5) * WIDTH as f32 - 0.5).round() as u32;
    let py = ((0.5 - ndc.y * 0.5) * HEIGHT as f32 - 0.5).round() as u32;
    (px.min(WIDTH - 1), py.min(HEIGHT - 1))
}

fn shade(material: Material, px: u32, py: u32) -> [u8; 4] {
    expected_color(scene::shade(
        material,
        plane_point(px, py),
        Vec3::Z,
        camera::EYE,
    ))
}

/// Snapshot 28: two draws share one mesh with per-object materials; one
/// shared group changes both; moving object 1 rewrites only its record.
#[test]
fn objects_share_geometry_but_swap_materials_and_poses() {
    if !gpu_context()
        .map(|ctx| ctx.adapter_features.contains(wgpu::Features::IMMEDIATES))
        .unwrap_or(false)
    {
        println!("skip: IMMEDIATES is not supported by this adapter");
        return;
    }
    let Some(ctx) = gpu_context_with(wgpu::DeviceDescriptor {
        label: Some("Scene objects verify device"),
        required_features: wgpu::Features::IMMEDIATES,
        required_limits: wgpu::Limits {
            max_immediate_size: 4,
            ..wgpu::Limits::default()
        },
        ..Default::default()
    }) else {
        println!("skip: no suitable GPU adapter");
        return;
    };
    let gpu = Gpu {
        device: ctx.device.clone(),
        queue: ctx.queue.clone(),
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
    };
    let mut sample = SceneObjects::init(&gpu).expect("init sample");
    sample.resize(WIDTH, HEIGHT);

    // Object centers project onto mirrored pixels (camera on x = 0 plane).
    let left = covering_pixel(-0.9, 0.0);
    let right = covering_pixel(0.9, 0.0);
    assert_eq!(left.0 + right.0, WIDTH - 1);
    assert_eq!(left.1, right.1);

    // Separate materials: each probe follows its own CPU-shaded material.
    let bytes = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    let left_pixel = pixel(&bytes, left.0, left.1);
    let right_pixel = pixel(&bytes, right.0, right.1);
    assert_color(
        left_pixel,
        shade(scene::WARM, left.0, left.1),
        "left object, warm",
    );
    assert_color(
        right_pixel,
        shade(scene::COOL, right.0, right.1),
        "right object, cool",
    );
    let delta: i32 = (0..3)
        .map(|c| (i32::from(left_pixel[c]) - i32::from(right_pixel[c])).abs())
        .sum();
    assert!(delta > 40, "distinct materials must differ, delta {delta}");

    // Shared material: both draws bind warm; mirrored probes shade identically.
    sample.set_shared_material(true);
    let bytes = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    let left_pixel = pixel(&bytes, left.0, left.1);
    let right_pixel = pixel(&bytes, right.0, right.1);
    assert_color(
        left_pixel,
        shade(scene::WARM, left.0, left.1),
        "left object, shared warm",
    );
    assert_color(
        right_pixel,
        shade(scene::WARM, right.0, right.1),
        "right object, shared warm",
    );
    assert_eq!(
        left_pixel, right_pixel,
        "one material, mirrored probes, equal codes"
    );

    // Moving object 1 rewrites only its record: old center -> background.
    sample.set_shared_material(false);
    sample.set_object1_moved(true);
    let bytes = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    let background = expected_color(Vec3::new(0.1, 0.1, 0.12));
    assert_color(
        pixel(&bytes, right.0, right.1),
        background,
        "old right center, background",
    );
    assert_color(
        pixel(&bytes, left.0, left.1),
        shade(scene::WARM, left.0, left.1),
        "left object stays",
    );
}
