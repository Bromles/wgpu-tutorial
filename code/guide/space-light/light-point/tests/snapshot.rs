//! Renders this chapter's article images into `docs/public/results`.
//! Run: VERIFY_SNAPSHOTS=1 cargo test -p light-point --test snapshot

use std::env::var;

use framework::Sample;
use light_point::sample::LightPoint;
use verify::gpu_context;
use verify::snapshot::{H, W, gpu_of, render, save, side_by_side};

#[test]
fn write_article_images() {
    if var("VERIFY_SNAPSHOTS").is_err() {
        println!("skip: set VERIFY_SNAPSHOTS=1 to rewrite article images");
        return;
    }
    let ctx = gpu_context().expect("adapter required for snapshots");

    let mut s = LightPoint::init(&gpu_of(&ctx)).unwrap();
    s.resize(W, H);
    let plain = render(&ctx, &mut s);
    s.set_spot_on(true);
    let cone = render(&ctx, &mut s);
    save(
        "light-point-cone.png",
        W * 2 + 8,
        H,
        &side_by_side(&plain, &cone, W, H),
    );
}
