//! Renders the article reference images into docs/public/results.
//! Run: cargo run -p foundations-verify --bin snapshots

use verify::{gpu_context, gpu_context_with, render_and_readback};
use framework::{Gpu, Sample};
use std::path::{Path, PathBuf};

const W: u32 = 768;
const H: u32 = 576;

fn results_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/public/results")
}

fn write_png(path: &Path, width: u32, height: u32, rgba: &[u8]) {
    let file = std::io::BufWriter::new(std::fs::File::create(path).expect("create png"));
    let mut encoder = png::Encoder::new(file, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("write png header");
    writer.write_image_data(rgba).expect("write png data");
}

/// Sanity guard: an article image must not be a flat fill.
fn check(name: &str, width: u32, height: u32, bytes: &[u8]) {
    let mut min = [255u8; 3];
    let mut max = [0u8; 3];
    let mut unique = std::collections::HashSet::new();
    for px in bytes.as_chunks::<4>().0 {
        for c in 0..3 {
            min[c] = min[c].min(px[c]);
            max[c] = max[c].max(px[c]);
        }
        unique.insert([px[0], px[1], px[2]]);
    }
    let spread: u32 = (0..3).map(|c| u32::from(max[c] - min[c])).sum();
    assert!(
        spread > 60,
        "{name}: degenerate image (spread {spread}, {} colors)",
        unique.len()
    );
    println!("{name}: {}x{} colors={}", width, height, unique.len());
}

fn save(name: &str, width: u32, height: u32, bytes: &[u8]) {
    let path = results_dir().join(name);
    write_png(&path, width, height, bytes);
    check(name, width, height, bytes);
}

fn side_by_side(a: &[u8], b: &[u8], width: u32, height: u32) -> Vec<u8> {
    let gap = 8;
    let full = width * 2 + gap;
    let mut out = vec![0u8; (full * height * 4) as usize];
    for y in 0..height {
        let src = (y * width * 4) as usize;
        let dst = (y * full * 4) as usize;
        out[dst..dst + (width * 4) as usize].copy_from_slice(&a[src..src + (width * 4) as usize]);
        let dst2 = dst + ((width + gap) * 4) as usize;
        out[dst2..dst2 + (width * 4) as usize].copy_from_slice(&b[src..src + (width * 4) as usize]);
    }
    out
}

fn crop_zoom(bytes: &[u8], x0: u32, y0: u32, cw: u32, ch: u32, zoom: u32) -> (Vec<u8>, u32, u32) {
    let mut out = vec![0u8; ((cw * zoom) * (ch * zoom) * 4) as usize];
    for dy in 0..ch * zoom {
        for dx in 0..cw * zoom {
            let sx = x0 + dx / zoom;
            let sy = y0 + dy / zoom;
            let src = ((sy * W + sx) * 4) as usize;
            let dst = ((dy * cw * zoom + dx) * 4) as usize;
            out[dst..dst + 4].copy_from_slice(&bytes[src..src + 4]);
        }
    }
    (out, cw * zoom, ch * zoom)
}

fn gpu_of(ctx: &verify::GpuContext) -> Gpu {
    Gpu {
        device: ctx.device.clone(),
        queue: ctx.queue.clone(),
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
    }
}

fn render<S: Sample>(ctx: &verify::GpuContext, sample: &mut S) -> Vec<u8> {
    render_and_readback(ctx, W, H, |encoder, view| {
        sample.draw(&gpu_of(ctx), encoder, view)
    })
}

fn main() {
    std::fs::create_dir_all(results_dir()).unwrap();

    let ctx = gpu_context().expect("adapter required for snapshots");

    // ---- space-light ----------------------------------------------------

    {
        use matrix_compose::sample::MatrixCompose;
        let mut s = MatrixCompose::init(&gpu_of(&ctx)).unwrap();
        s.resize(W, H);
        // The article's control pair: translate (0.25, 0), rotate pi/2.
        let t = glam::Mat4::from_translation(glam::Vec3::new(0.25, 0.0, 0.0));
        let r = glam::Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2);
        s.set_transform(t * r);
        let tr = render(&ctx, &mut s);
        s.set_transform(r * t);
        let rt = render(&ctx, &mut s);
        save(
            "matrix-compose-tr-rt.png",
            W * 2 + 8,
            H,
            &side_by_side(&tr, &rt, W, H),
        );
    }

    {
        use look_at::sample::LookAt;
        let mut s = LookAt::init(&gpu_of(&ctx)).unwrap();
        s.resize(W, H);
        save("look-at.png", W, H, &render(&ctx, &mut s));
    }

    {
        use ortho_perspective::sample::{OrthoPerspective, Projection};
        let mut s = OrthoPerspective::init(&gpu_of(&ctx)).unwrap();
        s.resize(W, H);
        s.set_projection(Projection::Perspective);
        let persp = render(&ctx, &mut s);
        s.set_projection(Projection::Orthographic);
        let ortho = render(&ctx, &mut s);
        save(
            "ortho-perspective-pair.png",
            W * 2 + 8,
            H,
            &side_by_side(&persp, &ortho, W, H),
        );
    }

    {
        use clip_viewport::sample::{ClipViewport, Interpolation};
        let mut s = ClipViewport::init(&gpu_of(&ctx)).unwrap();
        s.resize(W, H);
        s.set_interpolation(Interpolation::Perspective);
        let persp = render(&ctx, &mut s);
        s.set_interpolation(Interpolation::Linear);
        let linear = render(&ctx, &mut s);
        save(
            "clip-viewport-interpolation.png",
            W * 2 + 8,
            H,
            &side_by_side(&persp, &linear, W, H),
        );
    }

    {
        use depth_culling::sample::{Culling, DepthCulling, Order};
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

    {
        use camera_fly::sample::CameraFly;
        let mut s = CameraFly::init(&gpu_of(&ctx)).unwrap();
        s.resize(W, H);
        save("camera-fly.png", W, H, &render(&ctx, &mut s));
    }

    {
        use cube_uv::sample::CubeUv;
        let mut s = CubeUv::init(&gpu_of(&ctx)).unwrap();
        s.resize(W, H);
        let front = render(&ctx, &mut s);
        // FACE_VIEWS[1] = -Z (the back face), matching the caption.
        s.set_face(1);
        let back = render(&ctx, &mut s);
        save(
            "cube-uv-faces.png",
            W * 2 + 8,
            H,
            &side_by_side(&front, &back, W, H),
        );
    }

    {
        use lambert::sample::Lambert;
        let mut s = Lambert::init(&gpu_of(&ctx)).unwrap();
        s.resize(W, H);
        s.set_show_normals(false);
        let lit = render(&ctx, &mut s);
        s.set_show_normals(true);
        let normals = render(&ctx, &mut s);
        save(
            "lambert-normals.png",
            W * 2 + 8,
            H,
            &side_by_side(&lit, &normals, W, H),
        );
    }

    {
        use light_list::sample::LightList;
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

    {
        use light_point::sample::LightPoint;
        let mut s = LightPoint::init(&gpu_of(&ctx)).unwrap();
        s.resize(W, H);
        let plain = render(&ctx, &mut s);
        s.set_spot_on(true);
        let cone = render(&ctx, &mut s);
        save(
            "light-point-cone.png",
            W * 2 + 8,
            H,
            &side_by_side(&plain, &cone, W, H),
        );
    }

    {
        use normal_matrix::sample::NormalMatrix;
        let mut s = NormalMatrix::init(&gpu_of(&ctx)).unwrap();
        s.resize(W, H);
        s.set_scale(1.0);
        let one = render(&ctx, &mut s);
        s.set_scale(2.0);
        let two = render(&ctx, &mut s);
        save(
            "normal-matrix-scale.png",
            W * 2 + 8,
            H,
            &side_by_side(&one, &two, W, H),
        );
    }

    {
        use blinn_phong::params::Material;
        use blinn_phong::sample::BlinnPhong;
        use glam::Vec3;
        let mut s = BlinnPhong::init(&gpu_of(&ctx)).unwrap();
        s.resize(W, H);
        let full = render(&ctx, &mut s);
        s.set_material(Material {
            albedo: Vec3::splat(0.5),
            specular: Vec3::ZERO,
            shininess: 32.0,
        });
        let diffuse = render(&ctx, &mut s);
        s.set_material(Material {
            albedo: Vec3::ZERO,
            specular: Vec3::splat(0.7),
            shininess: 32.0,
        });
        let specular = render(&ctx, &mut s);
        let triple = W * 3 + 16;
        let mut out = vec![0u8; ((triple * H) * 4) as usize];
        for y in 0..H {
            let src = (y * W * 4) as usize;
            let dst = (y * triple * 4) as usize;
            out[dst..dst + (W * 4) as usize].copy_from_slice(&diffuse[src..src + (W * 4) as usize]);
            let d2 = dst + ((W + 8) * 4) as usize;
            out[d2..d2 + (W * 4) as usize].copy_from_slice(&specular[src..src + (W * 4) as usize]);
            let d3 = dst + ((W * 2 + 16) * 4) as usize;
            out[d3..d3 + (W * 4) as usize].copy_from_slice(&full[src..src + (W * 4) as usize]);
        }
        save("blinn-phong-terms.png", triple, H, &out);
    }

    {
        use blend_over::sample::BlendOver;
        let mut s = BlendOver::init(&gpu_of(&ctx)).unwrap();
        s.resize(W, H);
        save("blend-over.png", W, H, &render(&ctx, &mut s));
    }

    {
        use blend_order::sample::{BlendOrder, Order};
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

    {
        use triangle_motion::params::Params;
        use triangle_motion::sample::TriangleMotion;
        let mut s = TriangleMotion::init(&gpu_of(&ctx)).unwrap();
        s.resize(W, H);
        s.set_params(Params {
            translate: glam::Vec2::new(-0.3, -0.15),
            scale_angle: glam::Vec2::new(1.0, 0.0),
        });
        let left = render(&ctx, &mut s);
        s.set_params(Params {
            translate: glam::Vec2::new(0.3, 0.15),
            scale_angle: glam::Vec2::new(1.6, 1.2),
        });
        let right = render(&ctx, &mut s);
        save(
            "triangle-motion-poses.png",
            W * 2 + 8,
            H,
            &side_by_side(&left, &right, W, H),
        );
    }

    // ---- frame -----------------------------------------------------------

    {
        use compute_basics::sample::ComputeBasics;
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

    {
        use image_pipeline::sample::{ImagePipeline, OutputMode};
        let mut s = ImagePipeline::init(&gpu_of(&ctx)).unwrap();
        s.resize(W, H);
        s.set_mode(OutputMode::Fragment);
        let frag = render(&ctx, &mut s);
        s.set_mode(OutputMode::Compute);
        let comp = render(&ctx, &mut s);
        save(
            "image-pipeline-paths.png",
            W * 2 + 8,
            H,
            &side_by_side(&frag, &comp, W, H),
        );
    }

    {
        use shadow_mapping::sample::ShadowMapping;
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

    {
        use shadow_pcf::sample::ShadowPcf;
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
        let inv = shadow_pcf::scene::camera_view_proj().inverse();
        let unproject = |px: u32, py: u32, ndc_z: f32| -> glam::Vec3 {
            let ndc_x = (px as f32 + 0.5) / W as f32 * 2.0 - 1.0;
            let ndc_y = 1.0 - (py as f32 + 0.5) / H as f32 * 2.0;
            let clip = inv * glam::Vec4::new(ndc_x, ndc_y, ndc_z, 1.0);
            clip.truncate() / clip.w
        };
        let hits_cube = |origin: glam::Vec3, dir: glam::Vec3, t_max: f32| -> bool {
            let (mut t_enter, mut t_exit) = (0.0f32, t_max);
            for axis in 0..3 {
                let (lo, hi) = (
                    shadow_pcf::scene::CUBE_MIN[axis],
                    shadow_pcf::scene::CUBE_MAX[axis],
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
                        std::mem::swap(&mut near_t, &mut far_t);
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
        println!(
            "pcf edge crop: floor bbox=({x0},{y0})-({x1},{y1}) window={cw}x{ch} zoomed={zw}x{zh}"
        );
        save(
            "shadow-pcf-edge.png",
            zw * 2 + 8,
            zh,
            &side_by_side(&hard_zoom, &soft_zoom, zw, zh),
        );
    }

    {
        use msaa_resolve::sample::MsaaResolve;
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

    {
        use hdr_output::sample::HdrOutput;
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

    let ctx = gpu_context_with(wgpu::DeviceDescriptor {
        label: Some("Snapshots device"),
        required_features: wgpu::Features::IMMEDIATES,
        required_limits: wgpu::Limits {
            max_immediate_size: 4,
            ..wgpu::Limits::default()
        },
        ..Default::default()
    })
    .expect("IMMEDIATES adapter required for the full-frame snapshot");
    {
        use scene_objects::sample::SceneObjects;
        let mut s = SceneObjects::init(&gpu_of(&ctx)).unwrap();
        s.resize(W, H);
        save("scene-objects.png", W, H, &render(&ctx, &mut s));
    }
    {
        use full_frame::sample::FullFrame;
        let mut s = FullFrame::init(&gpu_of(&ctx)).unwrap();
        s.resize(W, H);
        save("full-frame.png", W, H, &render(&ctx, &mut s));
    }

    println!("all snapshots written to {}", results_dir().display());
}
