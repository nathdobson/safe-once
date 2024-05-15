use atomic::Atomic;
use parking_lot::lock_api::GuardSend;
use std::cell::UnsafeCell;
use std::fmt::{Debug, Formatter};
use std::mem;
use std::mem::MaybeUninit;
use std::num::NonZeroUsize;
use std::panic::{RefUnwindSafe, UnwindSafe};
use std::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{PoisonError, TryLockError};
use std::thread::{panicking, Thread};

use crate::api::raw::{RawFused, RawFusedState};
use parking_lot_core::{SpinWait, DEFAULT_PARK_TOKEN, DEFAULT_UNPARK_TOKEN};
// use crate::error::{LockError, PoisonError};
use crate::sync::state::State;
use crate::sync::thread_id::ThreadId;

#[derive(Debug)]
pub struct RawFusedLock {
    pub state: Atomic<State>,
}

impl RawFusedLock {
    #[cold]
    fn lock_checked_slow(&self, mut state: State) -> Result<RawFusedState, TryLockError<()>> {
        let tid = ThreadId::current();
        loop {
            if state.init() {
                return Ok(RawFusedState::Read);
            }
            if state.poison() {
                return Err(PoisonError::new(()).into());
            }
            if !state.locked() {
                assert_eq!(state, State::new());
                if let Err(new_state) = self.state.compare_exchange_weak(
                    state,
                    State::new().with_thread_id(tid).with_locked(true),
                    Relaxed,
                    Acquire,
                ) {
                    state = new_state;
                    continue;
                }
                return Ok(RawFusedState::Write);
            }
            if state.thread_id() == tid {
                return Err(TryLockError::WouldBlock);
            }
            if !state.parked() {
                if let Err(new_state) = self.state.compare_exchange_weak(
                    state,
                    state.with_parked(true),
                    Relaxed,
                    Acquire,
                ) {
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
            state = self.state.load(Ordering::Acquire);
        }
    }

    // #[cold]
    // fn get_checked_slow(&self, mut state: State) -> Result<RawFusedState, TryLockError<()>> {
    //     let tid = ThreadId::current();
    //     loop {
    //         if state.init() {
    //             return Ok(RawFusedState::Read);
    //         }
    //         if state.poison() {
    //             return Err(PoisonError::new(()).into());
    //         }
    //         if !state.locked() {
    //             assert_eq!(state, State::new());
    //             return Ok(RawFusedState::Write);
    //         }
    //         if state.thread_id() == tid {
    //             return Err(TryLockError::WouldBlock);
    //         }
    //         if !state.parked() {
    //             if let Err(new_state) = self.state.compare_exchange_weak(
    //                 state, state.with_parked(true), Relaxed, Acquire) {
    //                 state = new_state;
    //                 continue;
    //             }
    //             state = state.with_parked(true);
    //         }
    //         let addr = self as *const _ as usize;
    //         let validate = || {
    //             let state = self.state.load(Ordering::Relaxed);
    //             state.locked() && state.parked()
    //         };
    //         let before_sleep = || {};
    //         let timed_out = |_, _| unreachable!();
    //         unsafe {
    //             parking_lot_core::park(
    //                 addr,
    //                 validate,
    //                 before_sleep,
    //                 timed_out,
    //                 DEFAULT_PARK_TOKEN,
    //                 None,
    //             );
    //         }
    //         state = self.state.load(Ordering::Acquire);
    //     }
    // }

    #[cold]
    fn try_lock_checked_slow(
        &self,
        mut state: State,
    ) -> Result<Option<RawFusedState>, PoisonError<()>> {
        let tid = ThreadId::current();
        loop {
            if state.init() {
                return Ok(Some(RawFusedState::Read));
            }
            if state.poison() {
                return Err(PoisonError::new(()));
            }
            if !state.locked() {
                assert_eq!(state, State::new());
                if let Err(new_state) = self.state.compare_exchange_weak(
                    state,
                    State::new().with_thread_id(tid).with_locked(true),
                    Relaxed,
                    Acquire,
                ) {
                    state = new_state;
                    continue;
                }
                return Ok(Some(RawFusedState::Write));
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

unsafe impl RawFused for RawFusedLock {
    type GuardMarker = GuardSend;
    const UNLOCKED: Self = RawFusedLock {
        state: Atomic::new(State::new()),
    };
    const READ: Self = RawFusedLock {
        state: Atomic::new(State::new().with_init(true)),
    };
    const POISON: Self = RawFusedLock {
        state: Atomic::new(State::new().with_poison(true)),
    };

    fn write_checked(&self) -> Result<RawFusedState, TryLockError<()>> {
        let state = self.state.load(Ordering::Acquire);
        if state.init() {
            return Ok(RawFusedState::Read);
        }
        self.lock_checked_slow(state)
    }

    fn try_write_checked(&self) -> Result<Option<RawFusedState>, PoisonError<()>> {
        let state = self.state.load(Ordering::Acquire);
        if state.init() {
            return Ok(Some(RawFusedState::Read));
        }
        self.try_lock_checked_slow(state)
    }

    // fn read_checked(&self) -> Result<RawFusedState, TryLockError<()>> {
    //     let state = self.state.load(Ordering::Acquire);
    //     if state.init() {
    //         return Ok(RawFusedState::Read);
    //     }
    //     self.get_checked_slow(state)
    // }

    fn try_read_checked(&self) -> Result<RawFusedState, PoisonError<()>> {
        let state = self.state.load(Ordering::Acquire);
        if state.init() {
            return Ok(RawFusedState::Read);
        }
        if state.poison() {
            return Err(PoisonError::new(()));
        }
        return Ok(RawFusedState::Write);
    }

    unsafe fn unlock(&self) {
        self.unlock_impl(State::new());
    }

    unsafe fn unlock_fuse(&self) {
        self.unlock_impl(State::new().with_init(true));
    }

    unsafe fn unlock_poison(&self) {
        self.unlock_impl(State::new().with_poison(true));
    }

    fn try_get_mut(&mut self) -> Result<RawFusedState, PoisonError<()>> {
        let state = *self.state.get_mut();
        if state.init() {
            return Ok(RawFusedState::Read);
        }
        if state.poison() {
            return Err(PoisonError::new(()));
        }
        return Ok(RawFusedState::Write);
    }
}

impl RefUnwindSafe for RawFusedLock {}

impl UnwindSafe for RawFusedLock {}
