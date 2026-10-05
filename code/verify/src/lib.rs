//! Author verification harness: renders chapter snapshots offscreen and
//! compares them against the repository reference PNGs. A check run only
//! compares; `FOUNDATIONS_VERIFY_UPDATE=1` (re)writes references deliberately.

pub mod compare;
pub mod context;
pub mod png_io;
pub mod readback;
pub mod snapshot;

pub use compare::{ComparisonType, compare_reference, reference_path};
pub use context::{GpuContext, gpu_context, gpu_context_with};
pub use png_io::read_reference;
pub use readback::render_and_readback;
