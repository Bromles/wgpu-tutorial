const WIDTH: usize = 2;
const HEIGHT: usize = 2;
const CHANNELS: usize = 4;

// Rows run from top to bottom; each pixel stores R, G, B, A bytes.
const IMAGE: [u8; WIDTH * HEIGHT * CHANNELS] = [
    255, 0, 0, 255, 0, 255, 0, 255, // y = 0: red, green
    0, 0, 255, 255, 255, 255, 255, 255, // y = 1: blue, white
];

fn pixel_offset(x: usize,
y: usize) -> usize {
    assert!(x < WIDTH && y < HEIGHT, "pixel coordinates out of bounds");
    CHANNELS * (y * WIDTH + x)
}

fn set_red(image: &mut [u8; WIDTH * HEIGHT * CHANNELS],
x: usize,
y: usize,
red: u8) {
    image[pixel_offset(x, y)] = red;
}

// This quantizes a normalized channel; it does not encode linear RGB as sRGB.
fn quantize_channel(value: f32) -> u8 {
    assert!(value.is_finite(), "channel must be finite");
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn print_image(label: &str,
image: &[u8; WIDTH * HEIGHT * CHANNELS]) {
    println!("{label}");
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let offset = pixel_offset(x, y);
            println!(
                "({x},{y}) offset={offset:02} RGBA={:?}",
                &image[offset..offset + CHANNELS]
            );
        }
    }
}

fn main() {
    let mut image = IMAGE;
    print_image("before", &image);
    set_red(&mut image, 1, 1, quantize_channel(0.0));
    print_image("after", &image);
    println!("normalized 0.5 -> byte {}", quantize_channel(0.5));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixel_addresses_follow_rows_and_rgba_channels() {
        for (x, y, offset, rgba) in [
            (0, 0, 0, [255, 0, 0, 255]),
            (1, 0, 4, [0, 255, 0, 255]),
            (0, 1, 8, [0, 0, 255, 255]),
            (1, 1, 12, [255, 255, 255, 255]),
        ] {
            assert_eq!(pixel_offset(x, y), offset);
            assert_eq!(IMAGE[offset..offset + CHANNELS], rgba);
        }
    }

    #[test]
    fn mutation_changes_only_bottom_right_red() {
        let mut image = IMAGE;
        set_red(&mut image, 1, 1, quantize_channel(0.0));
        assert_eq!(&image[12..16], &[0, 255, 255, 255]);
        let changed: Vec<_> = (0..image.len())
            .filter(|&offset| image[offset] != IMAGE[offset])
            .collect();
        assert_eq!(changed, [12]);
    }

    #[test]
    fn quantizing_channels_clamps_and_rounds() {
        for (normalized, byte) in [
            (-0.25, 0),
            (0.0, 0),
            (0.25, 64),
            (0.5, 128),
            (0.75, 191),
            (1.0, 255),
            (1.25, 255),
        ] {
            assert_eq!(quantize_channel(normalized), byte);
        }
        // Every stored code survives normalization and requantization.
        for byte in 0..=255u8 {
            assert_eq!(quantize_channel(f32::from(byte) / 255.0), byte);
        }
    }
}
