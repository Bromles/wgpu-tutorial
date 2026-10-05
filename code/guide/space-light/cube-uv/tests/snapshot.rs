//! Renders this chapter's article images into `docs/public/results`.
//! Run: VERIFY_SNAPSHOTS=1 cargo test -p cube-uv --test snapshot

use std::env::var;

use cube_uv::sample::CubeUv;
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

    let mut s = CubeUv::init(&gpu_of(&ctx)).unwrap();
    s.resize(W, H);
    let front = render(&ctx, &mut s);
    // FACE_VIEWS[1] = -Z (the back face), matching the caption.
    s.set_face(1);
    let back = render(&ctx, &mut s);
    save(
        "cube-uv-faces.png",
        W * 2 + 8,
        H,
        &side_by_side(&front, &back, W, H),
    );
}
