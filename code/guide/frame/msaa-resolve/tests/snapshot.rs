//! Renders this chapter's article images into `docs/public/results`.
//! Run: VERIFY_SNAPSHOTS=1 cargo test -p msaa-resolve --test snapshot

use std::env::var;

use framework::Sample;
use msaa_resolve::sample::MsaaResolve;
use verify::gpu_context;
use verify::snapshot::{H, W, gpu_of, render, save, side_by_side};

#[test]
fn write_article_images() {
    if var("VERIFY_SNAPSHOTS").is_err() {
        println!("skip: set VERIFY_SNAPSHOTS=1 to rewrite article images");
        return;
    }
    let ctx = gpu_context().expect("adapter required for snapshots");

    let mut s = MsaaResolve::init(&gpu_of(&ctx)).unwrap();
    s.resize(W, H);
    s.set_msaa(false);
    let single = render(&ctx, &mut s);
    s.set_msaa(true);
    let multi = render(&ctx, &mut s);
    save(
        "msaa-resolve-1x-4x.png",
        W * 2 + 8,
        H,
        &side_by_side(&single, &multi, W, H),
    );
}
