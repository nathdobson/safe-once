//! A lazy initialization pattern where the initializer is supplied at construction.

use crate::api::fused::Fused;
use crate::api::raw::RawFused;
use std::cell::Cell;
use std::fmt::{Debug, Formatter};
use std::mem;
use std::ops::Deref;

enum State<T, F> {
    Callback(F),
    Value(T),
    Poisoned,
}

pub struct Lazy<R: RawFused, T, F = fn() -> T> {
    once: Fused<R, State<T, F>>,
}

impl<R: RawFused, T, F> Lazy<R, T, F> {
    pub const fn new(init: F) -> Self {
        Lazy {
            once: Fused::new(State::Callback(init)),
        }
    }
}

impl<R: RawFused, T, F: FnOnce() -> T> Deref for Lazy<R, T, F> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        match self
            .once
            .read_or_fuse(|x| match mem::replace(x, State::Poisoned) {
                State::Callback(f) => *x = State::Value(f()),
                State::Value(_) => unreachable!(),
                State::Poisoned => unreachable!(),
            }) {
            State::Callback(_) => unreachable!(),
            State::Value(x) => x,
            State::Poisoned => unreachable!(),
        }
    }
}

impl<R: RawFused, T: Default> Default for Lazy<R, T> {
    fn default() -> Self {
        Lazy::new(Default::default)
    }
}

impl<R: RawFused, T: Debug> Debug for Lazy<R, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        (**self).fmt(f)
    }
}
