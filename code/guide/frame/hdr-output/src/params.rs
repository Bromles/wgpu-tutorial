use encase::ShaderType;
use glam::{Mat4, Vec3};

/// Reflection parameters of the surface, not of the light or viewer.
#[derive(ShaderType, Debug, Clone, Copy, PartialEq)]
pub struct Material {
    pub albedo: Vec3,
    pub specular: Vec3,
    pub shininess: f32,
}

/// Direction toward the light; the intensity may leave the SDR range.
#[derive(ShaderType, Debug, Clone, Copy, PartialEq)]
pub struct Light {
    pub light_dir: Vec3,
    pub intensity: f32,
}

/// Frame uniforms; encase's 16-byte vec3 alignment gives 144 bytes.
#[derive(ShaderType, Debug, Clone, Copy, PartialEq)]
pub struct Params {
    pub view_proj: Mat4,
    /// World-space position of the eye; V = normalize(eye - P).
    pub eye: Vec3,
    pub light: Light,
    pub material: Material,
}

/// Tone pass uniform: exposure multiplier plus the diagnostic mode switch.
#[derive(ShaderType, Debug, Clone, Copy, PartialEq)]
pub struct ToneParams {
    pub exposure: f32,
    /// 0 = exposure + Reinhard, 1 = clipping view.
    pub mode: u32,
}

impl Material {
    pub const AMBIENT: f32 = 0.1;

    /// The chapter material of 25: matte grey base with a strong highlight.
    pub const CHAPTER: Material = Material {
        albedo: Vec3::splat(0.5),
        specular: Vec3::splat(0.7),
        shininess: 32.0,
    };
}

/// I-key presets: the chapter value and an HDR value past 1.0.
pub const INTENSITIES: [f32; 2] = [0.6, 4.0];

/// The exposure presets of the bracket keys, walked as a cycle.
pub const EXPOSURES: [f32; 4] = [0.5, 1.0, 2.0, 4.0];

impl Params {
    pub fn new(view_proj: Mat4,
    eye: Vec3,
    light: Light,
    material: Material) -> Self {
        Self {
            view_proj,
            eye,
            light,
            material,
        }
    }
}

impl ToneParams {
    /// Tone mapping with exposure, the default view.
    pub const TONE: u32 = 0;
    /// The diagnostic view: white wherever the linear frame exceeds 1.
    pub const CLIPPING: u32 = 1;
}
