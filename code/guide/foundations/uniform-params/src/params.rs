use encase::ShaderType;
use glam::Vec4;

/// WGSL contract: `tint` at offset 0, `gain` at 16, size 32; encase matches.
#[derive(ShaderType, Debug, Clone, Copy, PartialEq)]
pub struct Params {
    pub tint: Vec4,
    /// At offset 16; the trailing 12 bytes are uniform padding.
    pub gain: f32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            tint: Vec4::ONE,
            gain: 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use encase::UniformBuffer;

    #[test]
    fn params_layout_matches_wgsl_uniform_contract() {
        let mut buffer = UniformBuffer::new(Vec::<u8>::new());
        buffer.write(&Params::default()).unwrap();
        let bytes = buffer.into_inner();
        assert_eq!(
            bytes.len(),
            32,
            "uniform struct size rounds up to alignment 16"
        );
        let one = 1.0_f32.to_le_bytes();
        for offset in [0, 4, 8, 12, 16] {
            assert_eq!(&bytes[offset..offset + 4], &one, "field at offset {offset}");
        }
        assert!(bytes[20..32].iter().all(|&b| b == 0));
    }

    #[test]
    fn changed_gain_changes_only_its_own_bytes() {
        let mut buffer = UniformBuffer::new(Vec::<u8>::new());
        buffer
            .write(&Params {
                tint: Vec4::new(0.25, 0.5, 0.75, 1.0),
                gain: 0.5,
            })
            .unwrap();
        let bytes = buffer.into_inner();
        assert_eq!(bytes.len(), 32);
        assert_eq!(&bytes[16..20], &0.5_f32.to_le_bytes());
    }
}
