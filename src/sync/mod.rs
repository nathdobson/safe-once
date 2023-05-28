//! Implementations that are [Sync](::std::marker::Sync).

mod once_lock;
#[cfg(test)]
mod test;
mod state;
mod thread_id;

pub use once_lock::*;
use crate::lazy::Lazy;
use crate::once::Once;

pub type OnceLock<T> = Once<RawOnceLock, T>;
pub type LazyLock<T, F = fn() -> T> = Lazy<RawOnceLock, T, F>;
