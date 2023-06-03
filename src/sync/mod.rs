//! Implementations that are [Sync](::std::marker::Sync).

mod raw_fused_lock;
#[cfg(test)]
mod test;
mod state;
mod thread_id;

pub use raw_fused_lock::*;
use crate::api::fused::Fused;
use crate::api::lazy::Lazy;
use crate::api::once::Once;

pub type OnceLock<T> = Once<RawFusedLock, T>;
pub type LazyLock<T, F = fn() -> T> = Lazy<RawFusedLock, T, F>;
pub type FusedLock<T> = Fused<RawFusedLock, T>;