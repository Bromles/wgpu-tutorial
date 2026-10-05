//! Renders this chapter's article images into `docs/public/results`.
//! Run: VERIFY_SNAPSHOTS=1 cargo test -p normal-matrix --test snapshot

use std::env::var;

use framework::Sample;
use normal_matrix::sample::NormalMatrix;
use verify::gpu_context;
use verify::snapshot::{H, W, gpu_of, render, save, side_by_side};

#[test]
fn write_article_images() {
    if var("VERIFY_SNAPSHOTS").is_err() {
        println!("skip: set VERIFY_SNAPSHOTS=1 to rewrite article images");
        return;
    }
    let ctx = gpu_context().expect("adapter required for snapshots");

    let mut s = NormalMatrix::init(&gpu_of(&ctx)).unwrap();
    s.resize(W, H);
    s.set_scale(1.0);
    let one = render(&ctx, &mut s);
    s.set_scale(2.0);
    let two = render(&ctx, &mut s);
    save(
        "normal-matrix-scale.png",
        W * 2 + 8,
        H,
        &side_by_side(&one, &two, W, H),
    );
}
