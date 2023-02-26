use std::cell::Cell;
use std::default::default;
use std::fmt::{Debug, Formatter};
use std::ops::Deref;
use crate::once::Once;
use crate::RawOnce;

pub struct Lazy<R: RawOnce, T, F = fn() -> T> {
    once: Once<R, T>,
    init: Cell<Option<F>>,
}

impl<R: RawOnce, T, F> Lazy<R, T, F> {
    pub const fn new(init: F) -> Self {
        Lazy { once: Once::new(), init: Cell::new(Some(init)) }
    }
}

impl<R: RawOnce, T, F: FnOnce() -> T> Deref for Lazy<R, T, F> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.once.get_or_init(|| (self.init.take().unwrap())())
    }
}

impl<R: RawOnce, T: Default> Default for Lazy<R, T> {
    fn default() -> Self {
        Lazy::new(default)
    }
}

impl<R: RawOnce, T: Debug> Debug for Lazy<R, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        (**self).fmt(f)
    }
}

unsafe impl<R: RawOnce, T, F> Send for Lazy<R, T, F> where R: Send, F: Send, T: Send {}

unsafe impl<R: RawOnce, T, F> Sync for Lazy<R, T, F> where Once<R, T>: Sync {}