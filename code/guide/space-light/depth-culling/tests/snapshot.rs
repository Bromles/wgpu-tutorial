//! Renders this chapter's article images into `docs/public/results`.
//! Run: VERIFY_SNAPSHOTS=1 cargo test -p depth-culling --test snapshot

use depth_culling::sample::{Culling, DepthCulling, Order};
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

    let mut s = DepthCulling::init(&gpu_of(&ctx)).unwrap();
    s.resize(W, H);
    s.set_depth_enabled(true);
    s.set_culling(Culling::Off);
    s.set_order(Order::RedFirst);
    let depth = render(&ctx, &mut s);
    s.set_depth_enabled(false);
    let nodepth = render(&ctx, &mut s);
    save(
        "depth-culling-order.png",
        W * 2 + 8,
        H,
        &side_by_side(&depth, &nodepth, W, H),
    );
}
