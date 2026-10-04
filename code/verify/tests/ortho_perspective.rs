use verify::{gpu_context, render_and_readback};
use ortho_perspective::sample::{OrthoPerspective, Projection};
use ortho_perspective::texture::TEXELS;
use framework::{Gpu, Sample};

const WIDTH: u32 = 768;
const HEIGHT: u32 = 576;

fn pixel(bytes: &[u8], px: u32, py: u32) -> [u8; 4] {
    let offset = ((py * WIDTH + px) * 4) as usize;
    bytes[offset..offset + 4].try_into().unwrap()
}

/// The four quadrant colors; textureLoad returns whole texels, so pixels
/// match one exactly.
fn is_texel(px: [u8; 4]) -> bool {
    TEXELS
        .as_chunks::<4>()
        .0
        .iter()
        .any(|texel| px == [texel[0], texel[1], texel[2], texel[3]])
}

/// Horizontal extent of texture-colored pixels in a row band.
fn texture_column_span(bytes: &[u8], rows: std::ops::Range<u32>) -> Option<(u32, u32)> {
    let mut span: Option<(u32, u32)> = None;
    for py in rows {
        for px in 0..WIDTH {
            if is_texel(pixel(bytes, px, py)) {
                span = Some(match span {
                    None => (px, px),
                    Some((min, max)) => (min.min(px), max.max(px)),
                });
            }
        }
    }
    span
}

fn width_of(span: (u32, u32)) -> u32 {
    span.1 - span.0 + 1
}

/// Snapshot 19a: quads 2 m and 4 m away - perspective shows the near one
/// twice as wide, ortho keeps both equal.
#[test]
fn perspective_halves_and_orthography_keeps_the_far_quad() {
    let Some(ctx) = gpu_context() else {
        println!("skip: no suitable GPU adapter");
        return;
    };
    let gpu = Gpu {
        device: ctx.device.clone(),
        queue: ctx.queue.clone(),
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
    };
    let mut sample = OrthoPerspective::init(&gpu).expect("init sample");
    sample.resize(WIDTH, HEIGHT);

    sample.set_projection(Projection::Perspective);
    let bytes = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    // Near quad in the upper half, far in the lower; each band finds its own.
    let near = texture_column_span(&bytes, 0..HEIGHT / 2).expect("near quad pixels");
    let far = texture_column_span(&bytes, HEIGHT / 2..HEIGHT).expect("far quad pixels");
    let (near_width, far_width) = (width_of(near), width_of(far));
    assert!(
        (near_width as i32 - 2 * far_width as i32).abs() <= 4,
        "perspective: near width {near_width} should be twice far width {far_width}"
    );
    // Both quads are centered: span ends sum to WIDTH - 1.
    assert!(
        ((near.0 as i32) + (near.1 as i32) - (WIDTH as i32 - 1)).abs() <= 6,
        "near quad should be centered, got span {near:?}"
    );

    sample.set_projection(Projection::Orthographic);
    let bytes = render_and_readback(&ctx, WIDTH, HEIGHT, |encoder, view| {
        sample.draw(&gpu, encoder, view);
    });
    let near = texture_column_span(&bytes, 0..HEIGHT / 2).expect("near quad pixels");
    let far = texture_column_span(&bytes, HEIGHT / 2..HEIGHT).expect("far quad pixels");
    let (near_width, far_width) = (width_of(near), width_of(far));
    assert!(
        (near_width as i32 - far_width as i32).abs() <= 3,
        "orthographic: near width {near_width} should equal far width {far_width}"
    );
}
