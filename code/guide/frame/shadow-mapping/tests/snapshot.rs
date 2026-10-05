//! Renders this chapter's article images into `docs/public/results`.
//! Run: VERIFY_SNAPSHOTS=1 cargo test -p shadow-mapping --test snapshot

use std::env::var;

use framework::Sample;
use shadow_mapping::sample::ShadowMapping;
use verify::gpu_context;
use verify::snapshot::{H, W, gpu_of, render, save};

#[test]
fn write_article_images() {
    if var("VERIFY_SNAPSHOTS").is_err() {
        println!("skip: set VERIFY_SNAPSHOTS=1 to rewrite article images");
        return;
    }
    let ctx = gpu_context().expect("adapter required for snapshots");

    let mut s = ShadowMapping::init(&gpu_of(&ctx)).unwrap();
    s.resize(W, H);
    let on = render(&ctx, &mut s);
    save("shadow-mapping.png", W, H, &on);
    s.set_shadows(false);
    let off = render(&ctx, &mut s);
    let mut mask = on.clone();
    for i in 0..(W * H) as usize {
        if on[i * 4..i * 4 + 3] != off[i * 4..i * 4 + 3] {
            mask[i * 4..i * 4 + 4].copy_from_slice(&[255, 0, 0, 255]);
        }
    }
    save("shadow-mapping-visibility.png", W, H, &mask);
}
