//! Renders this chapter's article images into `docs/public/results`.
//! Run: VERIFY_SNAPSHOTS=1 cargo test -p shadow-pcf --test snapshot

use std::env::var;

use framework::Sample;
use std::mem::swap;

use glam::{Vec3, Vec4};
use shadow_pcf::sample::ShadowPcf;
use shadow_pcf::scene::{CUBE_MAX, CUBE_MIN, camera_view_proj};
use verify::gpu_context;
use verify::snapshot::{H, W, crop_zoom, gpu_of, render, save, side_by_side};

#[test]
fn write_article_images() {
    if var("VERIFY_SNAPSHOTS").is_err() {
        println!("skip: set VERIFY_SNAPSHOTS=1 to rewrite article images");
        return;
    }
    let ctx = gpu_context().expect("adapter required for snapshots");

    let mut s = ShadowPcf::init(&gpu_of(&ctx)).unwrap();
    s.resize(W, H);
    s.set_bias(0.0005);
    s.set_pcf(false);
    let hard_full = render(&ctx, &mut s);
    save("shadow-pcf.png", W, H, &hard_full);
    s.set_pcf(true);
    let soft_full = render(&ctx, &mut s);
    // Floor pixels by ray cast: colours cannot tell the floor from
    // same-albedo cube faces.
    let inv = camera_view_proj().inverse();
    let unproject = |px: u32, py: u32, ndc_z: f32| -> Vec3 {
        let ndc_x = (px as f32 + 0.5) / W as f32 * 2.0 - 1.0;
        let ndc_y = 1.0 - (py as f32 + 0.5) / H as f32 * 2.0;
        let clip = inv * Vec4::new(ndc_x, ndc_y, ndc_z, 1.0);
        clip.truncate() / clip.w
    };
    let hits_cube = |origin: Vec3, dir: Vec3, t_max: f32| -> bool {
        let (mut t_enter, mut t_exit) = (0.0f32, t_max);
        for axis in 0..3 {
            let (lo, hi) = (
                CUBE_MIN[axis],
                CUBE_MAX[axis],
            );
            if dir[axis].abs() < 1e-9 {
                if origin[axis] < lo || origin[axis] > hi {
                    return false;
                }
            } else {
                let (mut near_t, mut far_t) = (
                    (lo - origin[axis]) / dir[axis],
                    (hi - origin[axis]) / dir[axis],
                );
                if near_t > far_t {
                    swap(&mut near_t, &mut far_t);
                }
                t_enter = t_enter.max(near_t);
                t_exit = t_exit.min(far_t);
                if t_enter > t_exit {
                    return false;
                }
            }
        }
        t_enter < t_exit
    };
    let floor_mask = |px: u32, py: u32| -> bool {
        let near = unproject(px, py, 0.0);
        let dir = unproject(px, py, 1.0) - near;
        if dir.y >= 0.0 {
            return false;
        }
        let t = -near.y / dir.y;
        let point = near + dir * t;
        if point.x.abs() > 2.0 || point.z.abs() > 2.0 {
            return false;
        }
        !hits_cube(near, dir, t - 1e-4)
    };
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0u32, 0u32);
    for py in 0..H {
        for px in 0..W {
            let i = (py * W + px) as usize;
            if hard_full[i * 4..i * 4 + 3] == soft_full[i * 4..i * 4 + 3] {
                continue;
            }
            if !floor_mask(px, py) {
                continue;
            }
            x0 = x0.min(px);
            y0 = y0.min(py);
            x1 = x1.max(px);
            y1 = y1.max(py);
        }
    }
    assert!(
        x0 != u32::MAX,
        "no visible floor edge pixels found for the PCF crop"
    );
    let m = 12;
    let cx0 = x0.saturating_sub(m);
    let cy0 = y0.saturating_sub(m);
    let cw = (x1 + m).min(W - 1) - cx0 + 1;
    let ch = (y1 + m).min(H - 1) - cy0 + 1;
    let (hard_zoom, zw, zh) = crop_zoom(&hard_full, cx0, cy0, cw, ch, 3);
    let (soft_zoom, _, _) = crop_zoom(&soft_full, cx0, cy0, cw, ch, 3);
    println!("pcf edge crop: floor bbox=({x0},{y0})-({x1},{y1}) window={cw}x{ch} zoomed={zw}x{zh}");
    save(
        "shadow-pcf-edge.png",
        zw * 2 + 8,
        zh,
        &side_by_side(&hard_zoom, &soft_zoom, zw, zh),
    );
}
