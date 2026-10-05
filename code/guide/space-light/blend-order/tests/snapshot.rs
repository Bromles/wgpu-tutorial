//! Renders this chapter's article images into `docs/public/results`.
//! Run: VERIFY_SNAPSHOTS=1 cargo test -p blend-order --test snapshot

use blend_order::sample::{BlendOrder, Order};
use framework::Sample;
use verify::gpu_context;
use verify::snapshot::{H, W, gpu_of, render, save, side_by_side};

#[test]
fn write_article_images() {
  
use std::env::var;  if var("VERIFY_SNAPSHOTS").is_err() {
        println!("skip: set VERIFY_SNAPSHOTS=1 to rewrite article images");
        return;
    }
    let ctx = gpu_context().expect("adapter required for snapshots");

    let mut s = BlendOrder::init(&gpu_of(&ctx)).unwrap();
    s.resize(W, H);
    s.set_order(Order::RedFirst);
    let red = render(&ctx, &mut s);
    s.set_order(Order::GreenFirst);
    let green = render(&ctx, &mut s);
    save(
        "blend-order-flip.png",
        W * 2 + 8,
        H,
        &side_by_side(&red, &green, W, H),
    );
}
