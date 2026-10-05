use framework::{Gpu, Sample};
use full_frame::geometry::PANEL_Y;
use full_frame::sample::FullFrame;
use full_frame::scene;
use glam::Vec3;
use glam::camera::rh::proj::directx::perspective;
use glam::camera::rh::view::look_at_mat4;
use std::array::from_fn;
use verify::{gpu_context_with, render_and_readback};
use wgpu::{DeviceDescriptor, Features, Limits, TextureFormat};

const WIDTH: u32 = 768;
const HEIGHT: u32 = 576;

/// The HDR clear color, linear, as in the sample.
const CLEAR: [f32; 3] = [0.1, 0.1, 0.12];

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

fn reinhard(c: f32) -> f32 {
    c / (1.0 + c)
}

fn pixel(bytes: &[u8], px: u32, py: u32) -> [u8; 4] {
    let offset = ((py * WIDTH + px) * 4) as usize;
    bytes[offset..offset + 4].try_into().unwrap()
}

/// Pixel centre nearest the projection of a world point, fixed camera.
fn project(p: Vec3) -> (u32, u32) {
    let view = look_at_mat4(scene::EYE, scene::TARGET, Vec3::Y);
    let proj = perspective(
        scene::FOV_Y,
        WIDTH as f32 / HEIGHT as f32,
        scene::NEAR,
        scene::FAR,
    );
    let clip = (proj * view) * p.extend(1.0);
    let ndc = clip.truncate() / clip.w;
    let px = ((ndc.x + 1.0) / 2.0 * WIDTH as f32 - 0.5)
        .round()
        .clamp(0.0, WIDTH as f32 - 1.0);
    let py = ((1.0 - ndc.y) / 2.0 * HEIGHT as f32 - 0.5)
        .round()
        .clamp(0.0, HEIGHT as f32 - 1.0);
    (px as u32, py as u32)
}

/// Floor probes on the +Y face, both visible from EYE. The shadow strip is
/// x in [-0.5, 0.95], z in [-0.5, 0.5]; lit probe outside, shadowed inside.
const LIT_PROBE: Vec3 = Vec3::new(-1.5, 0.0, 1.2);
const SHADOW_PROBE: Vec3 = Vec3::new(0.7, 0.0, 0.35);
/// Panel centre; behind it the ray leaves the floor, so the backdrop is HDR.
const PANEL_PROBE: Vec3 = Vec3::new(0.0, PANEL_Y, 0.0);

/// Lambert diffuse then the full chain: HDR linear, exposure 1, Reinhard,
/// one sRGB encode.
fn floor_code(visible: f32, channel: usize) -> u8 {
    let diffuse = scene::LIGHT_DIR.y;
    let linear = scene::lambert_linear(scene::FLOOR_ALBEDO, diffuse, visible);
    expected_code(reinhard(linear[channel]))
}

/// Panel pixel: straight-alpha blend over the HDR background in linear
/// light, then tone mapping.
fn panel_code(channel: usize) -> u8 {
    let blended = scene::PANEL_COLOR[channel] * scene::PANEL_COLOR[3]
        + CLEAR[channel] * (1.0 - scene::PANEL_COLOR[3]);
    expected_code(reinhard(blended))
}

fn assert_channel(actual: [u8; 4], expected: [u8; 3], label: &str) {
    for channel in 0..3 {
        assert!(
            (i32::from(expected[channel]) - i32::from(actual[channel])).abs() <= 1,
            "{label} channel {channel}: expected {} got {}",
            expected[channel],
            actual[channel]
        );
    }
}

/// Snapshot 34: the frame composes shadow, transparency and tone mapping;
/// H=off relights only the shadowed probe, MSAA changes edges only.
#[test]
fn frame_composes_shadow_transparency_and_tone_map() {
    if !gpu_context_with(DeviceDescriptor {
        label: Some("Probe device"),
        required_features: Features::empty(),
        required_limits: Limits::default(),
        ..Default::default()
    })
    .map(|ctx| ctx.adapter_features.contains(Features::IMMEDIATES))
    .unwrap_or(false)
    {
        println!("skip: IMMEDIATES is not supported by this adapter");
        return;
    }
    let Some(ctx) = gpu_context_with(DeviceDescriptor {
        label: Some("Full frame verify device"),
        required_features: Features::IMMEDIATES,
        required_limits: Limits {
            max_immediate_size: 4,
            ..Limits::default()
        },
        ..Default::default()
    }) else {
        println!("skip: no suitable GPU adapter");
        return;
    };
    let gpu = Gpu {
        device: ctx.device.clone(),
        queue: ctx.queue.clone(),
        format: TextureFormat::Rgba8UnormSrgb,
    };
    let mut sample = FullFrame::init(&gpu).expect("init sample");
    sample.resize(WIDTH, HEIGHT);

    let bytes = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });

    // Cube top: visible, lit, unshadowed - full chain albedo*(ambient+intensity*d).
    let (px, py) = project(Vec3::new(0.0, 1.0, 0.0));
    let d = scene::LIGHT_DIR.y;
    let cube_top_expected: [u8; 3] = from_fn(|channel| {
        let linear = scene::CUBE_ALBEDO[channel] * (scene::AMBIENT + scene::INTENSITY * d);
        expected_code(reinhard(linear))
    });
    assert_channel(pixel(&bytes, px, py), cube_top_expected, "cube top face");

    // Lit floor: outside the shadow strip, full chain as the CPU mirror.
    let (px, py) = project(LIT_PROBE);
    let lit_expected: [u8; 3] = from_fn(|channel| floor_code(1.0, channel));
    assert_channel(pixel(&bytes, px, py), lit_expected, "lit floor probe");

    // Shadowed floor: direct term gone, ambient-only through the chain.
    let (px, py) = project(SHADOW_PROBE);
    let shadow_expected: [u8; 3] = from_fn(|channel| floor_code(0.0, channel));
    assert_channel(pixel(&bytes, px, py), shadow_expected, "shadowed floor probe");

    // Panel: blends over the HDR background in linear light, then one encode.
    let (px, py) = project(PANEL_PROBE);
    let panel_expected: [u8; 3] = from_fn(panel_code);
    assert_channel(pixel(&bytes, px, py), panel_expected, "panel probe");

    // Frame sanity: not empty, lit pixels, shadow toggle changes something.
    let mut lit_found = false;
    for offset in (0..bytes.len()).step_by(4) {
        let alpha = bytes[offset + 3];
        if alpha == 255 {
            let brightness = bytes[offset];
            if brightness > 100 {
                lit_found = true;
            }
        }
    }
    assert!(lit_found, "frame must contain lit pixels");
    let mut min_brightness = 255u8;
    for offset in (0..bytes.len()).step_by(4) {
        if bytes[offset + 3] == 255 && bytes[offset] < min_brightness {
            min_brightness = bytes[offset];
        }
    }
    assert!(
        min_brightness < 120,
        "frame must contain darker pixels than the lit floor"
    );

    sample.set_shadows(false);
    let bytes2 = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    let changed = bytes.iter().zip(&bytes2).filter(|(a, b)| a != b).count();
    assert!(changed > 0, "toggling shadows must change the frame");

    // M = 4x: MSAA changes only edges; flat interiors keep byte values.
    sample.set_shadows(true);
    sample.set_msaa(true);
    let bytes_msaa = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    let (cx, cy) = project(Vec3::new(0.0, 1.0, 0.0));
    let top_msaa = pixel(&bytes_msaa, cx, cy);
    for channel in 0..3 {
        assert!(
            (i32::from(cube_top_expected[channel]) - i32::from(top_msaa[channel])).abs() <= 1,
            "cube top under MSAA channel {channel}: expected {}, got {}",
            cube_top_expected[channel],
            top_msaa[channel]
        );
    }
    let mut changed_pixels = 0usize;
    for offset in (0..bytes.len()).step_by(4) {
        if bytes[offset..offset + 4] != bytes_msaa[offset..offset + 4] {
            changed_pixels += 1;
        }
    }
    let total = (WIDTH * HEIGHT) as usize;
    assert!(
        changed_pixels > 20,
        "4x MSAA must mix edge pixels, {changed_pixels} changed"
    );
    assert!(
        changed_pixels * 20 < total,
        "MSAA may only change edges, but {changed_pixels}/{total} pixels changed"
    );
}
