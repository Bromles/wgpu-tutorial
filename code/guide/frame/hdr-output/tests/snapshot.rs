//! Renders this chapter's article images into `docs/public/results`.
//! Run: VERIFY_SNAPSHOTS=1 cargo test -p hdr-output --test snapshot

use std::env::var;

use framework::Sample;
use hdr_output::sample::HdrOutput;
use verify::gpu_context;
use verify::snapshot::{H, W, gpu_of, render, save, side_by_side};

#[test]
fn write_article_images() {
    if var("VERIFY_SNAPSHOTS").is_err() {
        println!("skip: set VERIFY_SNAPSHOTS=1 to rewrite article images");
        return;
    }
    let ctx = gpu_context().expect("adapter required for snapshots");

    let mut s = HdrOutput::init(&gpu_of(&ctx)).unwrap();
    s.resize(W, H);
    // Chapter defaults: exposure index 1 (=1.0), HDR intensity.
    s.set_exposure(1);
    s.set_intensity(1);
    s.set_clipping(true);
    let clip = render(&ctx, &mut s);
    s.set_clipping(false);
    let reinhard = render(&ctx, &mut s);
    save(
        "hdr-output-clip-reinhard.png",
        W * 2 + 8,
        H,
        &side_by_side(&clip, &reinhard, W, H),
    );
}
