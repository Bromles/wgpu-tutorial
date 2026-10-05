//! Renders this chapter's article images into `docs/public/results`.
//! Run: VERIFY_SNAPSHOTS=1 cargo test -p triangle-motion --test snapshot

use std::env::var;

use framework::Sample;
use glam::Vec2;
use triangle_motion::params::Params;
use triangle_motion::sample::TriangleMotion;
use verify::gpu_context;
use verify::snapshot::{H, W, gpu_of, render, save, side_by_side};

#[test]
fn write_article_images() {
    if var("VERIFY_SNAPSHOTS").is_err() {
        println!("skip: set VERIFY_SNAPSHOTS=1 to rewrite article images");
        return;
    }
    let ctx = gpu_context().expect("adapter required for snapshots");

    let mut s = TriangleMotion::init(&gpu_of(&ctx)).unwrap();
    s.resize(W, H);
    s.set_params(Params {
        translate: Vec2::new(-0.3, -0.15),
        scale_angle: Vec2::new(1.0, 0.0),
    });
    let left = render(&ctx, &mut s);
    s.set_params(Params {
        translate: Vec2::new(0.3, 0.15),
        scale_angle: Vec2::new(1.6, 1.2),
    });
    let right = render(&ctx, &mut s);
    save(
        "triangle-motion-poses.png",
        W * 2 + 8,
        H,
        &side_by_side(&left, &right, W, H),
    );
}
