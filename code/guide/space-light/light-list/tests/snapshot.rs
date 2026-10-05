//! Renders this chapter's article images into `docs/public/results`.
//! Run: VERIFY_SNAPSHOTS=1 cargo test -p light-list --test snapshot

use std::env::var;

use framework::Sample;
use light_list::sample::LightList;
use verify::gpu_context;
use verify::snapshot::{H, W, gpu_of, render, save, side_by_side};

#[test]
fn write_article_images() {
    if var("VERIFY_SNAPSHOTS").is_err() {
        println!("skip: set VERIFY_SNAPSHOTS=1 to rewrite article images");
        return;
    }
    let ctx = gpu_context().expect("adapter required for snapshots");

    let mut s = LightList::init(&gpu_of(&ctx)).unwrap();
    s.resize(W, H);
    s.set_count(1);
    let one = render(&ctx, &mut s);
    s.set_count(3);
    let three = render(&ctx, &mut s);
    save(
        "light-list-counts.png",
        W * 2 + 8,
        H,
        &side_by_side(&one, &three, W, H),
    );
}
