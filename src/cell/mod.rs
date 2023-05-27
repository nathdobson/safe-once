mod once_cell;

pub use once_cell::*;
use crate::lazy::Lazy;
use crate::once::Once;

pub type OnceCell<T> = Once<RawOnceCell, T>;
pub type LazyCell<T, F = fn() -> T> = Lazy<RawOnceCell, T, F>;
