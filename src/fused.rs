use std::cell::UnsafeCell;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::panic::{RefUnwindSafe, UnwindSafe};
use std::sync::{PoisonError, TryLockError};
use std::thread::panicking;
use crate::{RawOnce, RawOnceState};

pub struct Fused<R: RawOnce, T> {
    raw: R,
    data: UnsafeCell<T>,
}

pub enum FusedEntry<'a, R: RawOnce, T> {
    Read(&'a T),
    Write(FusedGuard<'a, R, T>),
}

pub struct FusedGuard<'a, R: RawOnce, T> {
    fused: Option<&'a Fused<R, T>>,
    marker: PhantomData<(&'a mut T, R::GuardMarker)>,
}

impl<'a, R: RawOnce, T> FusedGuard<'a, R, T> {
    pub fn init(mut self) -> &'a T {
        unsafe {
            let once = self.fused.take().unwrap();
            once.raw.unlock_init();
            &*once.data.get()
        }
    }
}

impl<'a, R: RawOnce, T> FusedEntry<'a, R, T> {
    pub fn or_init(self, modify: impl FnOnce(&mut T)) -> &'a T {
        match self {
            FusedEntry::Read(x) => x,
            FusedEntry::Write(x) => {
                let mut x = x;
                modify(&mut *x);
                x.init()
            }
        }
    }
}

impl<'a, R: RawOnce, T> Deref for FusedGuard<'a, R, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.fused.unwrap().data.get() }
    }
}

impl<'a, R: RawOnce, T> DerefMut for FusedGuard<'a, R, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.fused.unwrap().data.get() }
    }
}

impl<R: RawOnce, T> Fused<R, T> {
    pub const fn new(x: T) -> Self {
        Fused {
            raw: R::UNINIT,
            data: UnsafeCell::new(x),
        }
    }
    pub const fn poisoned(x: T) -> Self {
        Fused {
            raw: R::POISON,
            data: UnsafeCell::new(x),
        }
    }
    unsafe fn make_entry(&self, raw: RawOnceState) -> FusedEntry<R, T> {
        match raw {
            RawOnceState::Vacant => FusedEntry::Write(FusedGuard { fused: Some(self), marker: PhantomData }),
            RawOnceState::Occupied => FusedEntry::Read(&*self.data.get()),
        }
    }
    pub fn lock_checked(&self) -> Result<FusedEntry<R, T>, TryLockError<()>> {
        unsafe {
            Ok(self.make_entry(self.raw.lock_checked()?))
        }
    }
    pub fn lock(&self) -> FusedEntry<R, T> { self.lock_checked().unwrap() }
    pub fn try_lock_checked(&self) -> Result<Option<FusedEntry<R, T>>, TryLockError<()>> {
        unsafe {
            Ok(self.raw.try_lock_checked()?.map(|e| self.make_entry(e)))
        }
    }
    pub fn try_lock(&self) -> Option<FusedEntry<R, T>> {
        self.try_lock_checked().unwrap()
    }
    pub fn get_or_init(&self, init: impl FnOnce(&mut T)) -> &T {
        self.get_or_init_checked(init).unwrap()
    }
    pub fn get_or_init_checked(&self, init: impl FnOnce(&mut T)) -> Result<&T, TryLockError<()>> {
        Ok(self.lock_checked()?.or_init(init))
    }
    pub fn try_get_checked(&self) -> Result<Option<&T>, PoisonError<()>> {
        unsafe {
            Ok(match self.raw.try_get_checked()? {
                RawOnceState::Vacant => None,
                RawOnceState::Occupied => Some(&*self.data.get())
            })
        }
    }
    pub fn get_checked(&self) -> Result<Option<&T>, TryLockError<()>> {
        unsafe {
            Ok(match self.raw.get_checked()? {
                RawOnceState::Vacant => None,
                RawOnceState::Occupied => Some(&*self.data.get())
            })
        }
    }
    pub fn try_get(&self) -> Option<&T> {
        self.try_get_checked().unwrap()
    }
    pub fn get(&self) -> Option<&T> {
        self.get_checked().unwrap()
    }
    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }
}

impl<R: RawOnce, T> From<T> for Fused<R, T> {
    fn from(value: T) -> Self {
        Fused { raw: R::INIT, data: UnsafeCell::new(value) }
    }
}

unsafe impl<R: RawOnce + Send, T: Send> Send for Fused<R, T> {}

unsafe impl<R: RawOnce + Send + Sync, T: Send + Sync> Sync for Fused<R, T> {}

impl<R: RawOnce + RefUnwindSafe + UnwindSafe, T: RefUnwindSafe + UnwindSafe> RefUnwindSafe for Fused<R, T> {}

impl<R: RawOnce + UnwindSafe, T: UnwindSafe> UnwindSafe for Fused<R, T> {}

// impl<R: RawOnce + Debug, T: Debug> Debug for Fused<R, T> {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         f.debug_struct("Once")
//             .field("raw", &self.raw)
//             .field("value", &self.try_get())
//             .finish()
//     }
// }

impl<R: RawOnce, T: Default> Default for Fused<R, T> {
    fn default() -> Self { Fused::new(T::default()) }
}

impl<'a, R: RawOnce, T> Drop for FusedGuard<'a, R, T> {
    fn drop(&mut self) {
        unsafe {
            if let Some(once) = self.fused {
                if panicking() {
                    once.raw.unlock_poison();
                } else {
                    once.raw.unlock_nopoison();
                }
            }
        }
    }
}


// impl<R: RawOnce, T: Clone> Clone for Fused<R, T> {
//     fn clone(&self) -> Self {
//         match self.try_get_checked() {
//             Ok(Some(x)) => Fused::from(x.clone()),
//             Ok(None) => Fused::new(),
//             Err(_) => Fused::poisoned(),
//         }
//     }
// }