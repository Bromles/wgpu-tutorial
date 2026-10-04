use draw_immediates::sample::DrawImmediates;
use foundations_verify::{ComparisonType, gpu_context, reference_path, render_and_readback};
use framework::{Gpu, Sample};

fn pixel(bytes: &[u8], width: u32, px: u32, py: u32) -> [u8; 4] {
    let offset = ((py * width + px) * 4) as usize;
    bytes[offset..offset + 4].try_into().unwrap()
}

/// Snapshot 11: two draws select their tints by immediate index; dropping the
/// second write makes both draws read index 0.
#[test]
fn immediates_select_tints_per_draw() {
    if !gpu_context()
        .map(|ctx| ctx.adapter_features.contains(wgpu::Features::IMMEDIATES))
        .unwrap_or(false)
    {
        println!("skip: IMMEDIATES is not supported by this adapter");
        return;
    }
    let Some(ctx) = foundations_verify::gpu_context_with(wgpu::DeviceDescriptor {
        label: Some("Immediates verify device"),
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
    let (width, height) = (768, 576);
    let gpu = Gpu {
        device: ctx.device.clone(),
        queue: ctx.queue.clone(),
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
    };
    let neutral =
        foundations_verify::read_reference(&reference_path("indexed-geometry.png"), width, height)
            .expect("indexed-geometry reference exists");
    let mut sample = DrawImmediates::init(&gpu).expect("init sample");
    let bytes = render_and_readback(&ctx, width, height, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });

    // Per-draw selection tints the second triangle; the first keeps natural.
    for px_py in [(280, 210), (480, 360)] {
        let natural = pixel(&neutral, width, px_py.0, px_py.1);
        let rendered = pixel(&bytes, width, px_py.0, px_py.1);
        let delta: i32 = (0..3)
            .map(|c| (i32::from(natural[c]) - i32::from(rendered[c])).abs())
            .sum();
        if px_py.0 < 384 {
            assert!(
                delta <= 2,
                "first draw keeps the neutral tint, delta {delta}"
            );
        } else {
            assert!(delta > 40, "second draw must be tinted, delta {delta}");
        }
    }

    let path = reference_path("draw-immediates.png");
    foundations_verify::compare_reference(
        &path,
        &ctx.adapter_info,
        width,
        height,
        &bytes,
        &[
            ComparisonType::Mean(0.02),
            ComparisonType::Percentile {
                percentile: 0.99,
                threshold: 0.35,
            },
        ],
    );

    // Both immediates zeroed: both draws read index 0, natural color back.
    sample.drop_second_selection();
    let bytes = render_and_readback(&ctx, width, height, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    let rendered = pixel(&bytes, width, 480, 360);
    let natural = pixel(&neutral, width, 480, 360);
    for channel in 0..3 {
        assert!(
            (i32::from(natural[channel]) - i32::from(rendered[channel])).abs() <= 2,
            "second draw must fall back to index 0"
        );
    }
}
