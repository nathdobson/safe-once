//! The core synchronization primitive that is shared by both Once* structs and Lazy* structs.

use std::sync::{PoisonError, TryLockError};

/// The state of a RawOnceState at the beginning of a call.
pub enum RawOnceState {
    /// Initialization is neither complete nor in progress.
    Vacant,
    /// Initialization is complete.
    Occupied,
}

/// A  RawOnce tracks the state of an external memory location, and guards access
/// to interior mutations of that location. The RawOnce can be in four states:
/// * UNINIT    - The memory location is not initialized.
/// * LOCKED    - One caller may choose to initialize the memory location.
/// * INIT      - The memory location is initialized.
/// * POISON    - The memory location is not initialized and never will be because a panic occurred.
///
/// Memory ordering guarantees:
/// Any operation that returns Occupied happens after the call to unlock_init.
/// Any operation that returns Poisoned happens after the call to unlock_poison.
pub unsafe trait RawOnce: 'static {
    /// The annotation that defines whether a OnceGuard is Send.
    type GuardMarker;
    const UNINIT: Self;
    const INIT: Self;
    const POISON: Self;

    /// * On UNINIT, transition to LOCKED and return Vacant.
    /// * On LOCKED, block or return WouldBlock if a deadlock is detected.
    /// * On INIT, return Occupied.
    /// * On POISON, return Poisoned.
    fn lock_checked(&self) -> Result<RawOnceState, TryLockError<()>>;

    /// * On UNINIT, transition to LOCKED and return Vacant.
    /// * On LOCKED, return WouldBlock.
    /// * On INIT, return Occupied.
    /// * On POISON, return Poisoned.
    fn try_lock_checked(&self) -> Result<Option<RawOnceState>, PoisonError<()>>;

    /// * On UNINIT, return Vacant.
    /// * On LOCKED, block or return WouldBlock if a deadlock is detected.
    /// * On INIT, return Occupied.
    /// * On POISON, return Poisoned.
    fn get_checked(&self) -> Result<RawOnceState, TryLockError<()>>;

    /// * On UNINIT, return Vacant.
    /// * On LOCKED, return WouldBlock.
    /// * On INIT, return Occupied.
    /// * On POISON, return Poisoned.
    fn try_get_checked(&self) -> Result<RawOnceState, PoisonError<()>>;

    /// Transition from LOCKED to UNINIT
    unsafe fn unlock_nopoison(&self);

    /// Transition from LOCKED to POISON
    unsafe fn unlock_poison(&self);

    /// Transition from LOCKED to INIT
    unsafe fn unlock_init(&self);
}
