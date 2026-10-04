use verify::{ComparisonType, gpu_context, reference_path, render_and_readback};
use mipmap_minification::sample::{LodClamp, MipmapMinification};
use framework::{Gpu, Sample};

fn pixel(bytes: &[u8], width: u32, px: u32, py: u32) -> [u8; 4] {
    let offset = ((py * width + px) * 4) as usize;
    bytes[offset..offset + 4].try_into().unwrap()
}

/// Snapshot 14 at phase 0: forced level 1 pre-filters to uniform gray 188;
/// forced level 0 shows raw aliasing with mostly extreme black/white pixels.
#[test]
fn mipmap_prefilter_reduces_aliasing() {
    let Some(ctx) = gpu_context() else {
        println!("skip: no suitable GPU adapter");
        return;
    };
    let (width, height) = (768, 576);
    let gpu = Gpu {
        device: ctx.device.clone(),
        queue: ctx.queue.clone(),
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
    };
    let mut sample = MipmapMinification::init(&gpu).expect("init sample");
    sample.set_elapsed(0.0);

    // Level 1 of the checkerboard averages 2x2 cells: uniform 0.5 light.
    sample.set_lod_clamp(LodClamp::Level1);
    let lod1 = render_and_readback(&ctx, width, height, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    for (px, py) in [(384, 288), (200, 200), (500, 380)] {
        let p = pixel(&lod1, width, px, py);
        for channel in &p[..3] {
            assert!(
                (188 - i32::from(*channel)).abs() <= 1,
                "level 1 expected gray 188 at ({px},{py}), got {p:?}"
            );
        }
    }

    // Level 0: raw checker - mostly extreme codes; phase re-pinned per render.
    sample.set_lod_clamp(LodClamp::Level0);
    sample.set_elapsed(0.0);
    let lod0 = render_and_readback(&ctx, width, height, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    let mut extreme = 0usize;
    let mut total = 0usize;
    for py in (100..500).step_by(4) {
        for px in (150..650).step_by(4) {
            let p = pixel(&lod0, width, px, py);
            total += 1;
            if p[0] < 20 || p[0] > 235 {
                extreme += 1;
            }
        }
    }
    let fraction = extreme as f32 / total as f32;
    assert!(
        fraction > 0.6,
        "level 0 expected mostly extreme pixels, got {fraction}"
    );

    // One second of animation shifts the UV by one texel, inverting the checker.
    sample.set_elapsed(1.0);
    let lod0_shifted = render_and_readback(&ctx, width, height, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    let mut differing = 0usize;
    let mut compared = 0usize;
    for py in (100..500).step_by(4) {
        for px in (150..650).step_by(4) {
            compared += 1;
            if pixel(&lod0, width, px, py) != pixel(&lod0_shifted, width, px, py) {
                differing += 1;
            }
        }
    }
    let moved = differing as f32 / compared as f32;
    assert!(
        moved > 0.8,
        "a one-texel phase shift must invert most of the checkerboard, moved {moved}"
    );

    for (name, bytes) in [("mipmap-lod1.png", &lod1), ("mipmap-lod0.png", &lod0)] {
        let path = reference_path(name);
        verify::compare_reference(
            &path,
            &ctx.adapter_info,
            width,
            height,
            bytes,
            &[
                ComparisonType::Mean(0.02),
                ComparisonType::Percentile {
                    percentile: 0.99,
                    threshold: 0.35,
                },
            ],
        );
    }
}
