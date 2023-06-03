//! A lazy initialization pattern where the initializer is supplied at access time.

use std::cell::UnsafeCell;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::mem;
use std::mem::MaybeUninit;
use std::panic::{RefUnwindSafe, UnwindSafe};
use std::sync::{PoisonError, TryLockError};
use std::thread::panicking;
use crate::{RawFused, RawFusedState};
use crate::fused::{Fused, FusedEntry, FusedGuard};

#[derive(Debug)]
pub struct Once<R: RawFused, T> {
    fused: Fused<R, MaybeUninit<T>>,
}

pub enum OnceEntry<'a, R: RawFused, T> {
    Occupied(&'a T),
    Vacant(OnceGuard<'a, R, T>),
}

pub struct OnceGuard<'a, R: RawFused, T>(FusedGuard<'a, R, MaybeUninit<T>>);

impl<'a, R: RawFused, T> OnceGuard<'a, R, T> {
    pub fn init(mut self, value: T) -> &'a T {
        unsafe {
            self.0.write(value);
            self.0.fuse().assume_init_ref()
        }
    }
}

impl<'a, R: RawFused, T> OnceEntry<'a, R, T> {
    pub fn or_init(self, value: impl FnOnce() -> T) -> &'a T {
        match self {
            OnceEntry::Occupied(x) => x,
            OnceEntry::Vacant(x) => x.init(value())
        }
    }
}

impl<R: RawFused, T> Once<R, T> {
    pub const fn new() -> Self {
        Once { fused: Fused::new(MaybeUninit::uninit()) }
    }
    pub const fn poisoned() -> Self {
        Once { fused: Fused::poisoned(MaybeUninit::uninit()) }
    }
    unsafe fn make_entry<'a>(&'a self, raw: FusedEntry<'a, R, MaybeUninit<T>>) -> OnceEntry<'a, R, T> {
        unsafe {
            match raw {
                FusedEntry::Read(read) => OnceEntry::Occupied(read.assume_init_ref()),
                FusedEntry::Write(write) => OnceEntry::Vacant(OnceGuard(write)),
            }
        }
    }
    pub fn lock_checked(&self) -> Result<OnceEntry<R, T>, TryLockError<()>> {
        unsafe {
            Ok(self.make_entry(self.fused.write_checked()?))
        }
    }
    pub fn lock(&self) -> OnceEntry<R, T> { self.lock_checked().unwrap() }
    pub fn try_lock_checked(&self) -> Result<Option<OnceEntry<R, T>>, TryLockError<()>> {
        unsafe {
            Ok(self.fused.try_write_checked()?.map(|e| self.make_entry(e)))
        }
    }
    pub fn try_lock(&self) -> Option<OnceEntry<R, T>> {
        self.try_lock_checked().unwrap()
    }
    pub fn get_or_init(&self, init: impl FnOnce() -> T) -> &T {
        self.get_or_init_checked(init).unwrap()
    }
    pub fn get_or_init_checked(&self, init: impl FnOnce() -> T) -> Result<&T, TryLockError<()>> {
        Ok(self.lock_checked()?.or_init(init))
    }
    pub fn try_get_checked(&self) -> Result<Option<&T>, PoisonError<()>> {
        unsafe {
            Ok(self.fused.try_read_checked()?.map(|x| x.assume_init_ref()))
        }
    }
    pub fn get_checked(&self) -> Result<Option<&T>, TryLockError<()>> {
        unsafe {
            Ok(self.fused.read_checked()?.map(|x| x.assume_init_ref()))
        }
    }
    pub fn try_get(&self) -> Option<&T> {
        self.try_get_checked().unwrap()
    }
    pub fn get(&self) -> Option<&T> {
        self.get_checked().unwrap()
    }
    fn into_inner_raw(self) -> Fused<R, MaybeUninit<T>> {
        unsafe {
            let result = ((&self.fused) as *const Fused<_, _>).read();
            mem::forget(self);
            result
        }
    }
    pub fn into_inner(mut self) -> Option<T> {
        unsafe {
            let (state, value) = self.into_inner_raw().into_inner();
            // self.fused = Fused::poisoned(MaybeUninit::uninit());
            match state {
                Ok(RawFusedState::Read) | Err(_) => { Some(value.assume_init_read()) }
                Ok(RawFusedState::Write) => None
            }
        }
    }
}

impl<R: RawFused, T> Drop for Once<R, T> {
    fn drop(&mut self) {
        unsafe {
            let (state, value) = self.fused.get_mut();
            match state {
                Ok(RawFusedState::Read) | Err(_) => value.assume_init_drop(),
                Ok(RawFusedState::Write) => {}
            }
        }
    }
}

impl<R: RawFused, T> From<T> for Once<R, T> {
    fn from(value: T) -> Self {
        Once { fused: Fused::new_read(MaybeUninit::new(value)) }
    }
}

unsafe impl<R: RawFused + Send, T: Send> Send for Once<R, T> {}

unsafe impl<R: RawFused + Send + Sync, T: Send + Sync> Sync for Once<R, T> {}

impl<R: RawFused + RefUnwindSafe + UnwindSafe, T: RefUnwindSafe + UnwindSafe> RefUnwindSafe for Once<R, T> {}

impl<R: RawFused + UnwindSafe, T: UnwindSafe> UnwindSafe for Once<R, T> {}

// impl<R: RawFused + Debug, T: Debug> Debug for Once<R, T> {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         f.debug_struct("Once")
//             .field("raw", &self.fu)
//             .field("value", &self.try_get())
//             .finish()
//     }
// }

impl<R: RawFused, T> Default for Once<R, T> {
    fn default() -> Self { Once::new() }
}

impl<R: RawFused, T: Clone> Clone for Once<R, T> {
    fn clone(&self) -> Self {
        match self.try_get_checked() {
            Ok(Some(x)) => Once::from(x.clone()),
            Ok(None) => Once::new(),
            Err(_) => Once::poisoned(),
        }
    }
}