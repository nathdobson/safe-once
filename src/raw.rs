use crate::{LockError, PoisonError};

pub enum RawOnceState { Occupied, Vacant }

pub unsafe trait RawOnce {
    type GuardMarker;
    const UNINIT: Self;
    const INIT: Self;
    const POISON: Self;
    fn lock_checked(&self) -> Result<RawOnceState, LockError>;
    fn try_lock_checked(&self) -> Result<Option<RawOnceState>, LockError>;
    fn get_checked(&self) -> Result<RawOnceState, PoisonError>;
    unsafe fn unlock_nopoison(&self);
    unsafe fn unlock_poison(&self);
    unsafe fn unlock_init(&self);
}
