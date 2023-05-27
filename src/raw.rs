// use crate::{LockError, PoisonError};

use std::sync::{PoisonError, TryLockError};

pub enum RawOnceState { Occupied, Vacant }

pub unsafe trait RawOnce: 'static {
    type GuardMarker;
    const UNINIT: Self;
    const INIT: Self;
    const POISON: Self;
    fn lock_checked(&self) -> Result<RawOnceState, TryLockError<()>>;
    fn try_lock_checked(&self) -> Result<Option<RawOnceState>, PoisonError<()>>;
    fn get_checked(&self) -> Result<RawOnceState, TryLockError<()>>;
    fn try_get_checked(&self) -> Result<RawOnceState, PoisonError<()>>;
    unsafe fn unlock_nopoison(&self);
    unsafe fn unlock_poison(&self);
    unsafe fn unlock_init(&self);
}
