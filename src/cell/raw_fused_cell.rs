use crate::api::raw::{RawFused, RawFusedState};
use parking_lot::lock_api::GuardNoSend;
use std::cell::{Cell, UnsafeCell};
use std::fmt::{Debug, Formatter};
use std::mem::MaybeUninit;
use std::sync::{PoisonError, TryLockError};
use std::thread::panicking;
// use crate::error::{LockError, PoisonError};

#[derive(Copy, Clone, Debug)]
enum State {
    Uninit,
    Initializing,
    Initialized,
    Poison,
}

#[derive(Debug)]
pub struct RawFusedCell(Cell<State>);

unsafe impl RawFused for RawFusedCell {
    type GuardMarker = GuardNoSend;
    const UNLOCKED: Self = RawFusedCell(Cell::new(State::Uninit));
    const READ: Self = RawFusedCell(Cell::new(State::Initialized));
    const POISON: Self = RawFusedCell(Cell::new(State::Poison));

    fn write_checked(&self) -> Result<RawFusedState, TryLockError<()>> {
        self.try_write_checked()?.ok_or(TryLockError::WouldBlock)
    }

    fn try_write_checked(&self) -> Result<Option<RawFusedState>, PoisonError<()>> {
        match self.0.get() {
            State::Uninit => {
                self.0.set(State::Initializing);
                Ok(Some(RawFusedState::Write))
            }
            State::Initializing => Ok(None),
            State::Initialized => Ok(Some(RawFusedState::Read)),
            State::Poison => Err(PoisonError::new(())),
        }
    }
    // fn read_checked(&self) -> Result<RawFusedState, TryLockError<()>> {
    //     Ok(self.try_read_checked()?)
    // }

    fn try_read_checked(&self) -> Result<RawFusedState, PoisonError<()>> {
        match self.0.get() {
            State::Uninit => Ok(RawFusedState::Write),
            State::Initializing => Ok(RawFusedState::Write),
            State::Initialized => Ok(RawFusedState::Read),
            State::Poison => Err(PoisonError::new(())),
        }
    }
    unsafe fn unlock(&self) {
        match self.0.get() {
            State::Initializing => self.0.set(State::Uninit),
            _ => panic!("Not already initializing"),
        }
    }
    unsafe fn unlock_fuse(&self) {
        match self.0.get() {
            State::Initializing => self.0.set(State::Initialized),
            _ => panic!("Not already initializing"),
        }
    }

    unsafe fn unlock_poison(&self) {
        match self.0.get() {
            State::Initializing => self.0.set(State::Poison),
            _ => panic!("Not already initializing"),
        }
    }

    fn try_get_mut(&mut self) -> Result<RawFusedState, PoisonError<()>> {
        match *self.0.get_mut() {
            State::Uninit => Ok(RawFusedState::Write),
            State::Initializing => Ok(RawFusedState::Write),
            State::Initialized => Ok(RawFusedState::Read),
            State::Poison => Err(PoisonError::new(())),
        }
    }
}
