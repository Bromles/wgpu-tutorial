use wgpu::TextureFormat;
use verify::{ComparisonType, compare_reference, gpu_context, reference_path, render_and_readback};
use framework::{Gpu, Sample};
use texture_load::sample::TextureLoad;

fn pixel(bytes: &[u8], width: u32, px: u32, py: u32) -> [u8; 4] {
    let offset = ((py * width + px) * 4) as usize;
    bytes[offset..offset + 4].try_into().unwrap()
}

/// Snapshot 13a: the four quadrants show the four known texels; textureLoad
/// decodes them back to the very codes of the chapter 01 array.
#[test]
fn texture_load_shows_known_texels() {
    let Some(ctx) = gpu_context() else {
        println!("skip: no suitable GPU adapter");
        return;
    };
    let (width, height) = (768, 576);
    let gpu = Gpu {
        device: ctx.device.clone(),
        queue: ctx.queue.clone(),
        format: TextureFormat::Rgba8UnormSrgb,
    };
    let mut sample = TextureLoad::init(&gpu).expect("init sample");
    let bytes = render_and_readback(&ctx, width, height, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });

    let expected = [
        ((200, 150), [255, 0, 0, 255]),     // top-left: red
        ((560, 150), [0, 255, 0, 255]),     // top-right: green
        ((200, 430), [0, 0, 255, 255]),     // bottom-left: blue
        ((560, 430), [255, 255, 255, 255]), // bottom-right: white
    ];
    for ((px, py), rgba) in expected {
        assert_eq!(
            pixel(&bytes, width, px, py),
            rgba,
            "quadrant at ({px},{py})"
        );
    }

    let path = reference_path("texture-load.png");
    compare_reference(
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
}
