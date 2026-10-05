//! Renders this chapter's article images into `docs/public/results`.
//! Run: VERIFY_SNAPSHOTS=1 cargo test -p compute-basics --test snapshot

use std::env::var;

use compute_basics::sample::ComputeBasics;
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

    let mut s = ComputeBasics::init(&gpu_of(&ctx)).unwrap();
    s.resize(W, H);
    let full = render(&ctx, &mut s);
    s.set_count(256);
    let repeat = render(&ctx, &mut s);
    save(
        "compute-basics-strip.png",
        W * 2 + 8,
        H,
        &side_by_side(&full, &repeat, W, H),
    );
}
