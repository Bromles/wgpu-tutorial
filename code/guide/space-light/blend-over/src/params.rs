use encase::ShaderType;
use glam::Mat4;
use glam::camera::rh::proj::directx::orthographic;

/// Ortho design frame half extents; 4:3 matches the window and the offscreen harness.
pub const HALF_X: f32 = 2.4;
pub const HALF_Y: f32 = 1.8;
/// The single plane of all quads, in the middle of the clip volume.
pub const Z: f32 = -1.0;
/// Ortho volume near/far: Z lands exactly at NDC depth 0.5.
pub const NEAR: f32 = 0.0;
pub const FAR: f32 = 2.0;

/// Fixed ortho camera, eye at the origin looking along -Z (chapter 19a).
pub fn ortho() -> Mat4 {
    orthographic(-HALF_X, HALF_X, -HALF_Y, HALF_Y, NEAR, FAR)
}

/// The only uniform of the snapshot; static, so it is uploaded once.
#[derive(ShaderType, Debug, Clone, Copy, PartialEq)]
pub struct Params {
    pub view_proj: Mat4,
}

/// Preset colors: A = opaque blue bg + red source; B = black bg + red/green sources at 0.5.
pub const BLUE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
pub const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
pub const RED: [f32; 4] = [1.0, 0.0, 0.0, 0.5];
pub const GREEN: [f32; 4] = [0.0, 1.0, 0.0, 0.5];

/// One over step, matching the pipeline's SrcAlpha / OneMinusSrcAlpha factors.
pub fn over_straight(src: [f32; 4], dst: [f32; 4]) -> [f32; 4] {
    let keep = 1.0 - src[3];
    [
        src[0] * src[3] + dst[0] * keep,
        src[1] * src[3] + dst[1] * keep,
        src[2] * src[3] + dst[2] * keep,
        src[3] + dst[3] * keep,
    ]
}

/// Converts straight to premultiplied: RGB weighted by alpha.
pub fn premultiply(color: [f32; 4]) -> [f32; 4] {
    [
        color[0] * color[3],
        color[1] * color[3],
        color[2] * color[3],
        color[3],
    ]
}

/// One over step in premultiplied form: only the destination needs (1 - a).
pub fn over_premultiplied(src: [f32; 4], dst: [f32; 4]) -> [f32; 4] {
    let keep = 1.0 - src[3];
    [
        src[0] + dst[0] * keep,
        src[1] + dst[1] * keep,
        src[2] + dst[2] * keep,
        src[3] + dst[3] * keep,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Preset A: red 0.5 over opaque blue gives (0.5, 0, 0.5) in both representations.
    #[test]
    fn preset_a_is_purple_in_both_representations() {
        let straight = over_straight(RED, BLUE);
        let premultiplied = over_premultiplied(premultiply(RED), premultiply(BLUE));
        let expected = [0.5, 0.0, 0.5];
        for c in 0..3 {
            assert!((straight[c] - expected[c]).abs() < 1e-6);
            assert!((premultiplied[c] - expected[c]).abs() < 1e-6);
        }
    }

    /// Compositing over an opaque background leaves the frame opaque.
    #[test]
    fn opaque_background_keeps_frame_alpha_at_one() {
        let a = over_straight(RED, BLUE);
        let b = over_premultiplied(premultiply(GREEN), premultiply(BLACK));
        assert!((a[3] - 1.0).abs() < 1e-6);
        assert!((b[3] - 1.0).abs() < 1e-6);
    }

    /// Preset B: red then green gives (0.25, 0.5, 0), the reverse (0.5, 0.25, 0).
    #[test]
    fn source_order_changes_the_result() {
        let red_first = over_premultiplied(
            premultiply(GREEN),
            over_premultiplied(premultiply(RED), premultiply(BLACK)),
        );
        let green_first = over_premultiplied(
            premultiply(RED),
            over_premultiplied(premultiply(GREEN), premultiply(BLACK)),
        );
        for (got, expected) in [
            (red_first, [0.25, 0.5, 0.0]),
            (green_first, [0.5, 0.25, 0.0]),
        ] {
            for c in 0..3 {
                assert!((got[c] - expected[c]).abs() < 1e-6);
            }
        }
    }
}
