//! Author verification harness: renders chapter snapshots offscreen and
//! compares them against the repository reference PNGs. A check run only
//! compares; `FOUNDATIONS_VERIFY_UPDATE=1` (re)writes references deliberately.

use std::path::{Path, PathBuf};

use wgpu::{
    Buffer, CommandEncoder, Extent3d, MapMode, PollType, TexelCopyBufferInfo,
    TexelCopyBufferLayout, TexelCopyTextureInfo, TextureAspect,
};

pub struct GpuContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub adapter_info: wgpu::AdapterInfo,
    /// Adapter's available features; the device only has the requested subset.
    pub adapter_features: wgpu::Features,
}

/// `None` only when no adapter exists; other failures panic, not skip.
pub fn gpu_context() -> Option<GpuContext> {
    gpu_context_with(wgpu::DeviceDescriptor {
        label: Some("Foundations verify device"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(),
        ..Default::default()
    })
}

/// Same, with example-specific device requirements.
pub fn gpu_context_with(device_descriptor: wgpu::DeviceDescriptor<'static>) -> Option<GpuContext> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        ..Default::default()
    })) else {
        // Genuinely no adapter: the only legitimate skip.
        return None;
    };
    let adapter_features = adapter.features();
    let (device, queue) = pollster::block_on(adapter.request_device(&device_descriptor))
        .unwrap_or_else(|error| {
            panic!(
                "adapter found, but the device request failed: {error}. \
             The test's required features/limits exceed what this adapter provides; \
             this is a test requirement problem, not a missing GPU"
            )
        });
    Some(GpuContext {
        device,
        queue,
        adapter_info: adapter.get_info(),
        adapter_features,
    })
}

/// Offscreen render + readback; returns tightly packed sRGB bytes. Any
/// width works: the 256-byte row rule is absorbed by the copy and compacted.
pub fn render_and_readback(
    ctx: &GpuContext,
    width: u32,
    height: u32,
    draw: impl FnOnce(&mut CommandEncoder, &wgpu::TextureView),
) -> Vec<u8> {
    const ROW_ALIGNMENT: u32 = 256;
    let tight_row = width * 4;
    let padded_row = tight_row.div_ceil(ROW_ALIGNMENT) * ROW_ALIGNMENT;
    let texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Verify target"),
        size: Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Verify readback"),
        size: u64::from(padded_row) * u64::from(height),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = ctx
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Verify encoder"),
        });
    draw(&mut encoder, &view);
    copy_texture_to_buffer(&mut encoder, &texture, &buffer, width, height, padded_row);
    ctx.queue.submit([encoder.finish()]);

    let slice = buffer.slice(..);
    let (sender, receiver) = std::sync::mpsc::channel();
    slice.map_async(MapMode::Read, move |result| {
        sender.send(result).expect("receiver alive");
    });
    ctx.device
        .poll(PollType::wait_indefinitely())
        .expect("device is not lost");
    receiver
        .recv()
        .expect("map callback ran")
        .expect("map succeeds");
    let padded = slice
        .get_mapped_range()
        .expect("buffer is mapped after poll")
        .to_vec();
    buffer.unmap();
    // Compact: drop the inter-row padding the copy had to insert.
    if padded_row == tight_row {
        return padded;
    }
    let mut bytes = Vec::with_capacity((tight_row * height) as usize);
    for row in 0..height as usize {
        let start = row * padded_row as usize;
        bytes.extend_from_slice(&padded[start..start + tight_row as usize]);
    }
    bytes
}

fn copy_texture_to_buffer(
    encoder: &mut CommandEncoder,
    texture: &wgpu::Texture,
    buffer: &Buffer,
    width: u32,
    height: u32,
    padded_row: u32,
) {
    encoder.copy_texture_to_buffer(
        TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: TextureAspect::All,
        },
        TexelCopyBufferInfo {
            buffer,
            layout: TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_row),
                rows_per_image: None,
            },
        },
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
}

/// FLIP error-map statistic that fails the test when exceeded.
#[derive(Debug, Clone, Copy)]
pub enum ComparisonType {
    /// Fails when the mean FLIP error is greater than this value.
    Mean(f32),
    /// Fails when the percentile is greater than the threshold.
    Percentile { percentile: f32, threshold: f32 },
}

/// Path of a reference image inside `docs/public/results`.
pub fn reference_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/public/results")
        .join(name)
}

/// Compares rendered bytes with the reference PNG; a missing reference fails.
/// `FOUNDATIONS_VERIFY_UPDATE=1` writes it; on mismatch, PNGs go to target/verify-artifacts.
pub fn compare_reference(
    path: &Path,
    adapter_info: &wgpu::AdapterInfo,
    width: u32,
    height: u32,
    rendered: &[u8],
    checks: &[ComparisonType],
) {
    if std::env::var("FOUNDATIONS_VERIFY_UPDATE").as_deref() == Ok("1") {
        write_png(path, width, height, rendered);
        println!("REFERENCE WRITTEN: {}", path.display());
        println!("Inspect it, then commit it; the next run will compare against it.");
        return;
    }
    if checks.is_empty() {
        panic!("no comparison checks configured");
    }
    if !path.exists() {
        panic!(
            "reference {} is missing. To create it, run again with \
             FOUNDATIONS_VERIFY_UPDATE=1, inspect the image, and commit it",
            path.display()
        );
    }
    let reference = read_png(path, width, height).unwrap_or_else(|| {
        panic!(
            "reference {} is not a {}x{} RGBA8 png",
            path.display(),
            width,
            height
        )
    });

    let reference_flip = nv_flip::FlipImageRgb8::with_data(width, height, &rgb(&reference));
    let test_flip = nv_flip::FlipImageRgb8::with_data(width, height, &rgb(rendered));
    let error_map = nv_flip::flip(
        reference_flip,
        test_flip,
        nv_flip::DEFAULT_PIXELS_PER_DEGREE,
    );
    let mut pool = nv_flip::FlipPool::from_image(&error_map);

    println!("comparing against {}", path.display());
    println!("  mean: {:.6}", pool.mean());
    for percentile in [25, 50, 75, 95, 99] {
        println!(
            "  {:>2}%: {:.6}",
            percentile,
            pool.get_percentile(percentile as f32 / 100.0, true)
        );
    }

    let mut all_passed = true;
    for check in checks {
        let passed = match *check {
            ComparisonType::Mean(limit) => {
                let value = pool.mean();
                println!("  mean {:.6} <= {}: {}", value, limit, value <= limit);
                value <= limit
            }
            ComparisonType::Percentile {
                percentile,
                threshold,
            } => {
                let value = pool.get_percentile(percentile, true);
                println!(
                    "  p{} {:.6} <= {}: {}",
                    (percentile * 100.0) as u32,
                    value,
                    threshold,
                    value <= threshold
                );
                value <= threshold
            }
        };
        all_passed &= passed;
    }

    let renderer = format!(
        "{}-{}-{}",
        adapter_info.backend, adapter_info.name, adapter_info.driver
    )
    .chars()
    .map(|ch| {
        if ch.is_ascii_alphanumeric() || ch == '-' {
            ch
        } else {
            '_'
        }
    })
    .collect::<String>();
    let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
    if !all_passed {
        // Diagnostics only on failure, into the repo-root target/.
        let artifacts =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/verify-artifacts");
        std::fs::create_dir_all(&artifacts).expect("create artifacts directory");
        let actual = artifacts.join(format!("{stem}-{renderer}-actual.png"));
        let difference = artifacts.join(format!("{stem}-{renderer}-difference.png"));
        write_png(&actual, width, height, rendered);
        let magma = error_map.apply_color_lut(&nv_flip::magma_lut()).to_vec();
        write_png(&difference, width, height, &rgba(&magma));
        assert!(all_passed, "image mismatch: see {}", difference.display());
    }
}

fn rgb(rgba: &[u8]) -> Vec<u8> {
    rgba.as_chunks::<4>()
        .0
        .iter()
        .flat_map(|c| &c[..3])
        .copied()
        .collect()
}

fn rgba(rgb: &[u8]) -> Vec<u8> {
    rgb.as_chunks::<3>()
        .0
        .iter()
        .flat_map(|c| [c[0], c[1], c[2], 255])
        .collect()
}

fn read_png(path: &Path, width: u32, height: u32) -> Option<Vec<u8>> {
    let data = std::fs::read(path).ok()?;
    let decoder = png::Decoder::new(std::io::Cursor::new(data));
    let mut reader = decoder.read_info().ok()?;
    let mut buffer = vec![0; reader.output_buffer_size().expect("buffer fits in memory")];
    let info = reader.next_frame(&mut buffer).ok()?;
    if info.width != width || info.height != height || info.color_type != png::ColorType::Rgba {
        return None;
    }
    Some(buffer)
}

/// Reads a reference image for pixel-by-pixel numeric comparisons.
pub fn read_reference(path: &Path, width: u32, height: u32) -> Option<Vec<u8>> {
    read_png(path, width, height)
}

fn write_png(path: &Path, width: u32, height: u32, rgba: &[u8]) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create results directory");
    }
    let file = std::io::BufWriter::new(std::fs::File::create(path).expect("create png"));
    let mut encoder = png::Encoder::new(file, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("write png header");
    writer.write_image_data(rgba).expect("write png data");
}
