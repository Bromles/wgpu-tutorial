//! Renders this chapter's article images into `docs/public/results`.
//! Run: VERIFY_SNAPSHOTS=1 cargo test -p blinn-phong --test snapshot

use blinn_phong::params::Material;
use glam::Vec3;
use blinn_phong::sample::BlinnPhong;
use std::env::var;

use framework::Sample;
use verify::gpu_context;
use verify::snapshot::{H, W, gpu_of, render, save};

#[test]
fn write_article_images() {
    if var("VERIFY_SNAPSHOTS").is_err() {
        println!("skip: set VERIFY_SNAPSHOTS=1 to rewrite article images");
        return;
    }
    let ctx = gpu_context().expect("adapter required for snapshots");

    let mut s = BlinnPhong::init(&gpu_of(&ctx)).unwrap();
    s.resize(W, H);
    let full = render(&ctx, &mut s);
    s.set_material(Material {
        albedo: Vec3::splat(0.5),
        specular: Vec3::ZERO,
        shininess: 32.0,
    });
    let diffuse = render(&ctx, &mut s);
    s.set_material(Material {
        albedo: Vec3::ZERO,
        specular: Vec3::splat(0.7),
        shininess: 32.0,
    });
    let specular = render(&ctx, &mut s);
    let triple = W * 3 + 16;
    let mut out = vec![0u8; ((triple * H) * 4) as usize];
    for y in 0..H {
        let src = (y * W * 4) as usize;
        let dst = (y * triple * 4) as usize;
        out[dst..dst + (W * 4) as usize].copy_from_slice(&diffuse[src..src + (W * 4) as usize]);
        let d2 = dst + ((W + 8) * 4) as usize;
        out[d2..d2 + (W * 4) as usize].copy_from_slice(&specular[src..src + (W * 4) as usize]);
        let d3 = dst + ((W * 2 + 16) * 4) as usize;
        out[d3..d3 + (W * 4) as usize].copy_from_slice(&full[src..src + (W * 4) as usize]);
    }
    save("blinn-phong-terms.png", triple, H, &out);
}
