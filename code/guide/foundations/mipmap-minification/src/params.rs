use encase::ShaderType;

/// UV transform of the chapter: uv' = uv * k + (phase, 0).
#[derive(ShaderType, Debug, Clone, Copy, PartialEq)]
pub struct Params {
    pub k: f32,
    pub phase: f32,
}

impl Params {
    pub fn new(k: f32, phase: f32) -> Self {
        Self { k, phase }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn k_gives_two_texels_per_pixel() {
        // 8 texels repeated k times across the 576-pixel quad -> k * 8 / 576 texels per pixel.
        let k = 144.0_f32;
        assert!((k * 8.0 / 576.0 - 2.0).abs() < 1e-6);
    }
}
