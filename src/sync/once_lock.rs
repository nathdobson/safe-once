use std::cell::UnsafeCell;
use std::fmt::{Debug, Formatter};
use std::mem;
use std::mem::MaybeUninit;
use std::num::NonZeroUsize;
use std::panic::{RefUnwindSafe, UnwindSafe};
use std::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread::{panicking, Thread};
use atomic::Atomic;
use parking_lot::lock_api::GuardSend;

use crate::{Once, OnceEntry};
use parking_lot_core::{DEFAULT_PARK_TOKEN, DEFAULT_UNPARK_TOKEN, SpinWait};
use crate::error::{LockError, PoisonError};
use crate::raw::{RawOnce, RawOnceState};
use crate::sync::state::State;
use crate::sync::thread_id::ThreadId;


#[derive(Debug)]
pub struct RawOnceLock {
    pub state: Atomic<State>,
}

pub type OnceLock<T> = Once<RawOnceLock, T>;

impl RawOnceLock {
    #[cold]
    fn lock_checked_slow(&self, mut state: State) -> Result<RawOnceState, LockError> {
        let tid = ThreadId::current();
        loop {
            if state.init() {
                return Ok(RawOnceState::Occupied);
            }
            if state.poison() {
                return Err(LockError::PoisonError);
            }
            if !state.locked() {
                assert_eq!(state, State::new());
                if let Err(new_state) = self.state.compare_exchange_weak(
                    state, State::new().with_thread_id(tid).with_locked(true), Relaxed, Acquire) {
                    state = new_state;
                    continue;
                }
                return Ok(RawOnceState::Vacant);
            }
            if state.thread_id() == tid {
                return Err(LockError::CycleError);
            }
            if !state.parked() {
                if let Err(new_state) = self.state.compare_exchange_weak(
                    state, state.with_parked(true), Relaxed, Acquire) {
                    state = new_state;
                    continue;
                }
                state = state.with_parked(true);
            }
            let addr = self as *const _ as usize;
            let validate = || {
                let state = self.state.load(Ordering::Relaxed);
                state.locked() && state.parked()
            };
            let before_sleep = || {};
            let timed_out = |_, _| unreachable!();
            unsafe {
                parking_lot_core::park(
                    addr,
                    validate,
                    before_sleep,
                    timed_out,
                    DEFAULT_PARK_TOKEN,
                    None,
                );
            }
        }
    }
    #[cold]
    fn try_lock_checked_slow(&self, mut state: State) -> Result<Option<RawOnceState>, LockError> {
        let tid = ThreadId::current();
        loop {
            if state.init() {
                return Ok(Some(RawOnceState::Occupied));
            }
            if state.poison() {
                return Err(LockError::PoisonError);
            }
            if !state.locked() {
                assert_eq!(state, State::new());
                if let Err(new_state) = self.state.compare_exchange_weak(
                    state, State::new().with_thread_id(tid).with_locked(true), Relaxed, Acquire) {
                    state = new_state;
                    continue;
                }
                return Ok(Some(RawOnceState::Vacant));
            }
            return Ok(None);
        }
    }

    fn unlock_impl(&self, new_state: State) {
        let old_state = self.state.swap(new_state, Release);
        if old_state.parked() {
            let addr = self as *const _ as usize;
            unsafe {
                parking_lot_core::unpark_all(addr, DEFAULT_UNPARK_TOKEN);
            }
        }
    }
}

unsafe impl RawOnce for RawOnceLock {
    type GuardMarker = GuardSend;
    const UNINIT: Self = RawOnceLock { state: Atomic::new(State::new()) };
    const INIT: Self = RawOnceLock { state: Atomic::new(State::new().with_init(true)) };
    const POISON: Self = RawOnceLock { state: Atomic::new(State::new().with_poison(true)) };

    fn lock_checked(&self) -> Result<RawOnceState, LockError> {
        let state = self.state.load(Ordering::Acquire);
        if state.init() {
            return Ok(RawOnceState::Occupied);
        }
        self.lock_checked_slow(state)
    }

    fn try_lock_checked(&self) -> Result<Option<RawOnceState>, LockError> {
        let state = self.state.load(Ordering::Acquire);
        if state.init() {
            return Ok(Some(RawOnceState::Occupied));
        }
        self.try_lock_checked_slow(state)
    }

    fn get_checked(&self) -> Result<RawOnceState, PoisonError> {
        let state = self.state.load(Ordering::Acquire);
        if state.init() {
            return Ok(RawOnceState::Occupied);
        }
        if state.poison() {
            return Err(PoisonError);
        }
        return Ok(RawOnceState::Vacant);
    }

    unsafe fn unlock_nopoison(&self) {
        self.unlock_impl(State::new());
    }

    unsafe fn unlock_init(&self) {
        self.unlock_impl(State::new().with_init(true));
    }

    unsafe fn unlock_poison(&self) {
        self.unlock_impl(State::new().with_poison(true));
    }
}

impl RefUnwindSafe for RawOnceLock {}

impl UnwindSafe for RawOnceLock {}