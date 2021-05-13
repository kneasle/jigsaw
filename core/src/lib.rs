//! Idiomatic Rust representations of commonly used primitives for Change Ringing compositions.

mod bell;
pub mod block;
pub mod call;
pub mod method;
pub mod place_not;
pub mod row;
mod stage;
mod utils;

pub use bell::Bell;
pub use block::{AnnotBlock, AnnotRow, Block};
pub use call::Call;
pub use method::Method;
pub use place_not::{PlaceNot, PnBlock};
pub use row::{vec_row::Row, InvalidRowError, RowTrait};
pub use stage::{IncompatibleStages, Stage};
pub use utils::run_len;
// Re-export the SIMD row if the feature is enabled
#[cfg(feature = "simd_row")]
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[cfg(target_feature = "ssse3")]
#[cfg(target_feature = "sse4.1")]
pub use row::simd::SimdRow;
