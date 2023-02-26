use std::cell::{Cell, UnsafeCell};
use std::fmt::{Debug, Formatter};
use std::mem::MaybeUninit;
use std::thread::panicking;
use parking_lot::lock_api::GuardNoSend;
use crate::error::{LockError, PoisonError};
use crate::once::Once;
use crate::raw::{RawOnce, RawOnceState};

#[derive(Copy, Clone, Debug)]
enum State {
    Uninit,
    Initializing,
    Initialized,
    Poison,
}

#[derive(Debug)]
pub struct RawOnceCell(Cell<State>);


unsafe impl RawOnce for RawOnceCell {
    type GuardMarker = GuardNoSend;
    const UNINIT: Self = RawOnceCell(Cell::new(State::Uninit));
    const INIT: Self = RawOnceCell(Cell::new(State::Initialized));
    const POISON: Self = RawOnceCell(Cell::new(State::Poison));

    fn lock_checked(&self) -> Result<RawOnceState, LockError> {
        self.try_lock_checked()?.ok_or(LockError::CycleError)
    }

    fn try_lock_checked(&self) -> Result<Option<RawOnceState>, PoisonError> {
        match self.0.get() {
            State::Uninit => {
                self.0.set(State::Initializing);
                Ok(Some(RawOnceState::Vacant))
            }
            State::Initializing =>
                Ok(None),
            State::Initialized =>
                Ok(Some(RawOnceState::Occupied)),
            State::Poison =>
                Err(PoisonError),
        }
    }
    fn get_checked(&self) -> Result<RawOnceState, LockError> {
        Ok(self.try_get_checked()?)
    }

    fn try_get_checked(&self) -> Result<RawOnceState, PoisonError> {
        match self.0.get() {
            State::Uninit => Ok(RawOnceState::Vacant),
            State::Initializing => Ok(RawOnceState::Vacant),
            State::Initialized => Ok(RawOnceState::Occupied),
            State::Poison => Err(PoisonError),
        }
    }
    unsafe fn unlock_nopoison(&self) {
        match self.0.get() {
            State::Initializing => self.0.set(State::Uninit),
            _ => panic!("Not already initializing"),
        }
    }
    unsafe fn unlock_init(&self) {
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
}