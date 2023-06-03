use std::cell::UnsafeCell;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::panic::{RefUnwindSafe, UnwindSafe};
use std::sync::{PoisonError, TryLockError};
use std::thread::panicking;
use crate::{RawFused, RawFusedState};

// A mutex that can be made permanently read-only.
pub struct Fused<R: RawFused, T> {
    raw: R,
    data: UnsafeCell<T>,
}

// The result of trying to lock a Fused.
pub enum FusedEntry<'a, R: RawFused, T> {
    // The Fused is read-only and this is a reference to the underlying object.
    Read(&'a T),
    // The Fused is write-locked and this is a guard for mutating the underlying object.
    Write(FusedGuard<'a, R, T>),
}

// A guard for a write-lock of a Fused.
pub struct FusedGuard<'a, R: RawFused, T> {
    fused: Option<&'a Fused<R, T>>,
    marker: PhantomData<(&'a mut T, R::GuardMarker)>,
}

impl<'a, R: RawFused, T> FusedGuard<'a, R, T> {
    // Make this Fused read-only.
    pub fn fuse(mut self) -> &'a T {
        unsafe {
            let once = self.fused.take().unwrap();
            once.raw.unlock_fuse();
            &*once.data.get()
        }
    }
}

impl<'a, R: RawFused, T> FusedEntry<'a, R, T> {
    // Apply a modifier if writeable, and then make read-only
    pub fn or_fuse(self, modify: impl FnOnce(&mut T)) -> &'a T {
        match self {
            FusedEntry::Read(x) => x,
            FusedEntry::Write(x) => {
                let mut x = x;
                modify(&mut *x);
                x.fuse()
            }
        }
    }
}

impl<'a, R: RawFused, T> Deref for FusedGuard<'a, R, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.fused.unwrap().data.get() }
    }
}

impl<'a, R: RawFused, T> DerefMut for FusedGuard<'a, R, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.fused.unwrap().data.get() }
    }
}

impl<R: RawFused, T> Fused<R, T> {
    /// Construct a mutable Fused.
    pub const fn new(x: T) -> Self {
        Fused {
            raw: R::UNLOCKED,
            data: UnsafeCell::new(x),
        }
    }

    pub const fn new_read(x: T) -> Self {
        Fused {
            raw: R::READ,
            data: UnsafeCell::new(x),
        }
    }

    /// Construct an immutable Fused that causes and error when locking.
    pub const fn poisoned(x: T) -> Self {
        Fused {
            raw: R::POISON,
            data: UnsafeCell::new(x),
        }
    }
    unsafe fn make_entry(&self, raw: RawFusedState) -> FusedEntry<R, T> {
        match raw {
            RawFusedState::Write => FusedEntry::Write(FusedGuard { fused: Some(self), marker: PhantomData }),
            RawFusedState::Read => FusedEntry::Read(&*self.data.get()),
        }
    }
    /// Attempt to obtain a write lock and block if necessary.
    pub fn write_checked(&self) -> Result<FusedEntry<R, T>, TryLockError<()>> {
        unsafe {
            Ok(self.make_entry(self.raw.write_checked()?))
        }
    }
    /// Attempt to obtain a write lock and block if necessary. Panics if poisoned or deadlocked.
    pub fn write(&self) -> FusedEntry<R, T> { self.write_checked().unwrap() }
    /// Attempt to obtain a write lock without blocking.
    pub fn try_write_checked(&self) -> Result<Option<FusedEntry<R, T>>, TryLockError<()>> {
        unsafe {
            Ok(self.raw.try_write_checked()?.map(|e| self.make_entry(e)))
        }
    }
    /// Attempt to obtain a write lock without blocking. Panics if poisoned or deadlocked.
    pub fn try_write(&self) -> Option<FusedEntry<R, T>> {
        self.try_write_checked().unwrap()
    }
    /// If this is writeable, obtain a write lock, apply the modifier, make readable, and then
    /// return a reference. Otherwise just return the reference. 
    pub fn read_or_fuse_checked(&self, modify: impl FnOnce(&mut T)) -> Result<&T, TryLockError<()>> {
        Ok(self.write_checked()?.or_fuse(modify))
    }
    /// If this is writeable, obtain a write lock, apply the modifier, make readable, and then
    /// return a reference. Otherwise just return the reference. Panics if poisoned or deadlocked.
    pub fn read_or_fuse(&self, modify: impl FnOnce(&mut T)) -> &T {
        self.read_or_fuse_checked(modify).unwrap()
    }
    /// If this is read-only, return a reference to the underlying object. Does not block.
    pub fn try_read_checked(&self) -> Result<Option<&T>, PoisonError<()>> {
        unsafe {
            Ok(match self.raw.try_read_checked()? {
                RawFusedState::Write => None,
                RawFusedState::Read => Some(&*self.data.get())
            })
        }
    }
    /// If this is read-only, return a reference to the underlying object. Does not block.
    /// Will panic if poisoned or deadlocked.
    pub fn try_read(&self) -> Option<&T> {
        self.try_read_checked().unwrap()
    }
    pub fn read_checked(&self) -> Result<Option<&T>, TryLockError<()>> {
        unsafe {
            Ok(match self.raw.read_checked()? {
                RawFusedState::Write => None,
                RawFusedState::Read => Some(&*self.data.get())
            })
        }
    }
    pub fn read(&self) -> Option<&T> {
        self.read_checked().unwrap()
    }
    pub fn get_mut(&mut self) -> (Result<RawFusedState, PoisonError<()>>, &mut T) {
        (self.raw.try_get_mut(), self.data.get_mut())
    }
    pub fn into_inner(mut self) -> (Result<RawFusedState, PoisonError<()>>, T) {
        let state = self.raw.try_get_mut();
        (state, self.data.into_inner())
    }
}

unsafe impl<R: RawFused + Send, T: Send> Send for Fused<R, T> {}

unsafe impl<R: RawFused + Send + Sync, T: Send + Sync> Sync for Fused<R, T> {}

impl<R: RawFused + RefUnwindSafe + UnwindSafe, T: RefUnwindSafe + UnwindSafe> RefUnwindSafe for Fused<R, T> {}

impl<R: RawFused + UnwindSafe, T: UnwindSafe> UnwindSafe for Fused<R, T> {}

impl<R: RawFused + Debug, T: Debug> Debug for Fused<R, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Fused")
            .field("raw", &self.raw)
            .field("value", &self.try_read())
            .finish()
    }
}

impl<R: RawFused, T: Default> Default for Fused<R, T> {
    fn default() -> Self { Fused::new(T::default()) }
}

impl<'a, R: RawFused, T> Drop for FusedGuard<'a, R, T> {
    fn drop(&mut self) {
        unsafe {
            if let Some(fused) = self.fused {
                if panicking() {
                    fused.raw.unlock_poison();
                } else {
                    fused.raw.unlock();
                }
            }
        }
    }
}
