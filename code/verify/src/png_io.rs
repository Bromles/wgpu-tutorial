//! Reference PNG input/output.

use std::fs::{File, create_dir_all, read};
use std::io::{BufWriter, Cursor};
use std::path::Path;

use png::{BitDepth, ColorType, Decoder, Encoder};

pub(crate) fn read_png(path: &Path,
width: u32,
height: u32) -> Option<Vec<u8>> {
    let data = read(path).ok()?;
    let decoder = Decoder::new(Cursor::new(data));
    let mut reader = decoder.read_info().ok()?;
    let mut buffer = vec![0; reader.output_buffer_size().expect("buffer fits in memory")];
    let info = reader.next_frame(&mut buffer).ok()?;
    if info.width != width || info.height != height || info.color_type != ColorType::Rgba {
        return None;
    }
    Some(buffer)
}

/// Reads a reference image for pixel-by-pixel numeric comparisons.
pub fn read_reference(path: &Path,
width: u32,
height: u32) -> Option<Vec<u8>> {
    read_png(path, width, height)
}

pub(crate) fn write_png(path: &Path,
width: u32,
height: u32,
rgba: &[u8]) {
    if let Some(parent) = path.parent() {
        create_dir_all(parent).expect("create results directory");
    }
    let file = BufWriter::new(File::create(path).expect("create png"));
    let mut encoder = Encoder::new(file, width, height);
    encoder.set_color(ColorType::Rgba);
    encoder.set_depth(BitDepth::Eight);
    let mut writer = encoder.write_header().expect("write png header");
    writer.write_image_data(rgba).expect("write png data");
}
