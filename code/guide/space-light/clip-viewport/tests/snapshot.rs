//! Renders this chapter's article images into `docs/public/results`.
//! Run: VERIFY_SNAPSHOTS=1 cargo test -p clip-viewport --test snapshot

use clip_viewport::sample::{ClipViewport, Interpolation};
use std::env::var;

use framework::Sample;
use verify::gpu_context;
use verify::snapshot::{H, W, gpu_of, render, save, side_by_side};

#[test]
fn write_article_images() {
if var("VERIFY_SNAPSHOTS").is_err() {
        println!("skip: set VERIFY_SNAPSHOTS=1 to rewrite article images");
        return;
    }
    let ctx = gpu_context().expect("adapter required for snapshots");

    let mut s = ClipViewport::init(&gpu_of(&ctx)).unwrap();
    s.resize(W, H);
    s.set_interpolation(Interpolation::Perspective);
    let persp = render(&ctx, &mut s);
    s.set_interpolation(Interpolation::Linear);
    let linear = render(&ctx, &mut s);
    save(
        "clip-viewport-interpolation.png",
        W * 2 + 8,
        H,
        &side_by_side(&persp, &linear, W, H),
    );
}
