//! FLIP-based comparison of a rendered frame against the reference PNG.

use std::env::var;
use std::fs::create_dir_all;
use std::path::{Path, PathBuf};

use nv_flip::DEFAULT_PIXELS_PER_DEGREE;
use wgpu::AdapterInfo;

use crate::png_io::{read_png, write_png};
use nv_flip::{FlipImageRgb8, FlipPool, flip, magma_lut};

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
    adapter_info: &AdapterInfo,
    width: u32,
    height: u32,
    rendered: &[u8],
    checks: &[ComparisonType],
) {
    if var("FOUNDATIONS_VERIFY_UPDATE").as_deref() == Ok("1") {
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

    let reference_flip = FlipImageRgb8::with_data(width, height, &rgb(&reference));
    let test_flip = FlipImageRgb8::with_data(width, height, &rgb(rendered));
    let error_map = flip(reference_flip, test_flip, DEFAULT_PIXELS_PER_DEGREE);
    let mut pool = FlipPool::from_image(&error_map);

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
        let artifacts = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/verify-artifacts");
        create_dir_all(&artifacts).expect("create artifacts directory");
        let actual = artifacts.join(format!("{stem}-{renderer}-actual.png"));
        let difference = artifacts.join(format!("{stem}-{renderer}-difference.png"));
        write_png(&actual, width, height, rendered);
        let magma = error_map.apply_color_lut(&magma_lut()).to_vec();
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
