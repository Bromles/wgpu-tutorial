//! Renders this chapter's article images into `docs/public/results`.
//! Run: VERIFY_SNAPSHOTS=1 cargo test -p lambert --test snapshot

use std::env::var;

use framework::Sample;
use lambert::sample::Lambert;
use verify::gpu_context;
use verify::snapshot::{H, W, gpu_of, render, save, side_by_side};

#[test]
fn write_article_images() {
    if var("VERIFY_SNAPSHOTS").is_err() {
        println!("skip: set VERIFY_SNAPSHOTS=1 to rewrite article images");
        return;
    }
    let ctx = gpu_context().expect("adapter required for snapshots");

    let mut s = Lambert::init(&gpu_of(&ctx)).unwrap();
    s.resize(W, H);
    s.set_show_normals(false);
    let lit = render(&ctx, &mut s);
    s.set_show_normals(true);
    let normals = render(&ctx, &mut s);
    save(
        "lambert-normals.png",
        W * 2 + 8,
        H,
        &side_by_side(&lit, &normals, W, H),
    );
}
