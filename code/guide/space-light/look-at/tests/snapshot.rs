//! Renders this chapter's article images into `docs/public/results`.
//! Run: VERIFY_SNAPSHOTS=1 cargo test -p look-at --test snapshot

use std::env::var;

use framework::Sample;
use look_at::sample::LookAt;
use verify::gpu_context;
use verify::snapshot::{H, W, gpu_of, render, save};

#[test]
fn write_article_images() {
    if var("VERIFY_SNAPSHOTS").is_err() {
        println!("skip: set VERIFY_SNAPSHOTS=1 to rewrite article images");
        return;
    }
    let ctx = gpu_context().expect("adapter required for snapshots");

    let mut s = LookAt::init(&gpu_of(&ctx)).unwrap();
    s.resize(W, H);
    save("look-at.png", W, H, &render(&ctx, &mut s));
}
