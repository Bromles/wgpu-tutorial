//! Renders this chapter's article images into `docs/public/results`.
//! Run: VERIFY_SNAPSHOTS=1 cargo test -p scene-objects --test snapshot

use std::env::var;

use framework::Sample;
use scene_objects::sample::SceneObjects;
use verify::gpu_context_with;
use verify::snapshot::{H, W, gpu_of, render, save};
use wgpu::{DeviceDescriptor, Features, Limits};

#[test]
fn write_article_images() {
    if var("VERIFY_SNAPSHOTS").is_err() {
        println!("skip: set VERIFY_SNAPSHOTS=1 to rewrite article images");
        return;
    }
    let ctx = gpu_context_with(DeviceDescriptor {
        label: Some("Snapshots device"),
        required_features: Features::IMMEDIATES,
        required_limits: Limits {
            max_immediate_size: 4,
            ..Limits::default()
        },
        ..Default::default()
    })
    .expect("IMMEDIATES adapter required for this snapshot");

    let mut s = SceneObjects::init(&gpu_of(&ctx)).unwrap();
    s.resize(W, H);
    save("scene-objects.png", W, H, &render(&ctx, &mut s));
}
