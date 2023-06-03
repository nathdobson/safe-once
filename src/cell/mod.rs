//! Implementations that are not [Sync](::std::marker::Sync).

mod raw_fused_cell;

pub use raw_fused_cell::*;
use crate::api::fused::Fused;
use crate::api::lazy::Lazy;
use crate::api::once::Once;

pub type OnceCell<T> = Once<RawFusedCell, T>;
pub type LazyCell<T, F = fn() -> T> = Lazy<RawFusedCell, T, F>;
pub type FusedCell<T> = Fused<RawFusedCell, T>;