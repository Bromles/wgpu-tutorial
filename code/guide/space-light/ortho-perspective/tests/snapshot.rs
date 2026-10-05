//! Renders this chapter's article images into `docs/public/results`.
//! Run: VERIFY_SNAPSHOTS=1 cargo test -p ortho-perspective --test snapshot

use std::env::var;

use framework::Sample;
use ortho_perspective::sample::{OrthoPerspective, Projection};
use verify::gpu_context;
use verify::snapshot::{H, W, gpu_of, render, save, side_by_side};

#[test]
fn write_article_images() {
    if var("VERIFY_SNAPSHOTS").is_err() {
        println!("skip: set VERIFY_SNAPSHOTS=1 to rewrite article images");
        return;
    }
    let ctx = gpu_context().expect("adapter required for snapshots");

    let mut s = OrthoPerspective::init(&gpu_of(&ctx)).unwrap();
    s.resize(W, H);
    s.set_projection(Projection::Perspective);
    let persp = render(&ctx, &mut s);
    s.set_projection(Projection::Orthographic);
    let ortho = render(&ctx, &mut s);
    save(
        "ortho-perspective-pair.png",
        W * 2 + 8,
        H,
        &side_by_side(&persp, &ortho, W, H),
    );
}
