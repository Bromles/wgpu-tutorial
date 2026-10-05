//! Renders this chapter's article images into `docs/public/results`.
//! Run: VERIFY_SNAPSHOTS=1 cargo test -p matrix-compose --test snapshot

use std::env::var;
use std::f32::consts::FRAC_PI_2;

use glam::{Mat4, Vec3};

use framework::Sample;
use matrix_compose::sample::MatrixCompose;
use verify::gpu_context;
use verify::snapshot::{H, W, gpu_of, render, save, side_by_side};

#[test]
fn write_article_images() {
    if var("VERIFY_SNAPSHOTS").is_err() {
        println!("skip: set VERIFY_SNAPSHOTS=1 to rewrite article images");
        return;
    }
    let ctx = gpu_context().expect("adapter required for snapshots");

    let mut s = MatrixCompose::init(&gpu_of(&ctx)).unwrap();
    s.resize(W, H);
    // The article's control pair: translate (0.25, 0), rotate pi/2.
    let t = Mat4::from_translation(Vec3::new(0.25, 0.0, 0.0));
    let r = Mat4::from_rotation_z(FRAC_PI_2);
    s.set_transform(t * r);
    let tr = render(&ctx, &mut s);
    s.set_transform(r * t);
    let rt = render(&ctx, &mut s);
    save(
        "matrix-compose-tr-rt.png",
        W * 2 + 8,
        H,
        &side_by_side(&tr, &rt, W, H),
    );
}
