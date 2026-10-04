use foundations_verify::{gpu_context, render_and_readback};
use hdr_output::params::EXPOSURES;
use hdr_output::sample::HdrOutput;
use framework::{Gpu, Sample};

const WIDTH: u32 = 768;
const HEIGHT: u32 = 576;

const ALBEDO: f32 = 0.5;
const SPECULAR: f32 = 0.7;
const AMBIENT: f32 = 0.1;
/// Specular exponent, reserved for the pinned exact probes.
#[allow(dead_code)]
const SHININESS: f32 = 32.0;

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

/// Blinn-Phong mirrored on the CPU for the exact frame centre: the camera
/// looks at the plane centre, so V = L and the highlight peaks.
fn center_linear(intensity: f32) -> f32 {
    let diffuse = 1.0;
    let spec = 1.0;
    ALBEDO * (AMBIENT + intensity * diffuse) + SPECULAR * (intensity * spec)
}

/// Snapshot 33: the centre pixel must equal the CPU mirror pushed through
/// exposure and Reinhard; the HDR intensity pushes the pre-tonemap value over 1.
#[test]
fn tone_map_matches_the_cpu_mirror() {
    let Some(ctx) = gpu_context() else {
        println!("skip: no suitable GPU adapter");
        return;
    };
    let gpu = Gpu {
        device: ctx.device.clone(),
        queue: ctx.queue.clone(),
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
    };
    let mut sample = HdrOutput::init(&gpu).expect("init sample");
    sample.resize(WIDTH, HEIGHT);

    // Exposure 1.0, chapter intensity 0.6: linear 0.77 in, Reinhard out.
    sample.set_exposure(EXPOSURES.iter().position(|&e| e == 1.0).unwrap());
    sample.set_intensity(0);
    let bytes = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    let center = pixel(&bytes, WIDTH / 2, HEIGHT / 2);
    let expected = expected_code(reinhard(center_linear(0.6)));
    assert!(
        (i32::from(expected) - i32::from(center[0])).abs() <= 1,
        "exposure 1: expected {expected} got {}",
        center[0]
    );

    // Exposure 2: the multiplier acts before the operator, Reinhard(2c).
    sample.set_exposure(EXPOSURES.iter().position(|&e| e == 2.0).unwrap());
    let bytes = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    let center = pixel(&bytes, WIDTH / 2, HEIGHT / 2);
    let expected = expected_code(reinhard(2.0 * center_linear(0.6)));
    assert!(
        (i32::from(expected) - i32::from(center[0])).abs() <= 1,
        "exposure 2: expected {expected} got {}",
        center[0]
    );

    // HDR intensity 4.0: the centre is linear 4.85 before tone mapping.
    sample.set_exposure(EXPOSURES.iter().position(|&e| e == 1.0).unwrap());
    sample.set_intensity(1);
    sample.set_clipping(true);
    let bytes = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    let center = pixel(&bytes, WIDTH / 2, HEIGHT / 2);
    let corner = pixel(&bytes, 8, 8);
    assert_eq!(center[0], 255, "clipping view marks the blown highlight");
    assert_eq!(corner[0], 0, "clipping view keeps SDR pixels black");

    // Without the diagnostic the same HDR frame compresses below 255.
    sample.set_clipping(false);
    let bytes = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    let center = pixel(&bytes, WIDTH / 2, HEIGHT / 2);
    let expected = expected_code(reinhard(center_linear(4.0)));
    assert!(
        (i32::from(expected) - i32::from(center[0])).abs() <= 1,
        "HDR tone mapped: expected {expected} got {}",
        center[0]
    );
    assert!(center[0] < 255, "Reinhard never reaches 1.0");
}
