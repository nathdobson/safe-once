//! The core synchronization primitive that is shared by both Once* structs and Lazy* structs.

use std::sync::{PoisonError, TryLockError};

/// The state of a RawFused at the beginning of a call.
pub enum RawFusedState {
    /// The object is still mutable.
    Write,
    /// The object is now immutable.
    Read,
}

/// A RawFused is a lock similar to a mutex that can be "fused" and permanently converted to a
/// read-only mode. A RawFused implicitly guards an object, but the location of that object is
/// defined by the caller.
///
/// A RawFused can be in one of four states:
/// * UNLOCKED  - No caller may access the object.
/// * WRITE     - Exactly one caller may mutate the object.
/// * READ      - All callers may read the object.
/// * POISON    - All callers may read the object. The object may be in
///               an inconsistent state due to a panic.
///
pub unsafe trait RawFused: 'static {
    /// The annotation that defines whether a guard is Send.
    type GuardMarker;
    const UNLOCKED: Self;
    const READ: Self;
    const POISON: Self;

    /// Attempt to obtain a write lock, blocking if necessary.
    /// * On UNLOCKED, transition to WRITE and return Write.
    /// * On WRITE, block or return WouldBlock if a deadlock is detected.
    /// * On READ, return Read.
    /// * On POISON, return Poisoned.
    fn write_checked(&self) -> Result<RawFusedState, TryLockError<()>>;

    /// Attempt to obtain a write lock, but do not block.
    /// * On UNLOCKED, transition to WRITE and return Write.
    /// * On WRITE, return WouldBlock.
    /// * On READ, return Read.
    /// * On POISON, return Poisoned.
    fn try_write_checked(&self) -> Result<Option<RawFusedState>, PoisonError<()>>;

    /// Attempt to use an existing read lock, blocking if there is a write lock
    /// * On UNLOCKED, return Write.
    /// * On WRITE, block or return WouldBlock if a deadlock is detected.
    /// * On READ, return Read.
    /// * On POISON, return Poisoned.
    // fn read_checked(&self) -> Result<RawFusedState, TryLockError<()>>;

    /// Attempt to use an existing read lock, but do not block
    /// * On UNLOCKED, return Write.
    /// * On LOCKED, return WouldBlock.
    /// * On INIT, return Read.
    /// * On POISON, return Poisoned.
    fn try_read_checked(&self) -> Result<RawFusedState, PoisonError<()>>;

    /// Transition from WRITE to UNLOCKED. Other states cause undefined behavior.
    unsafe fn unlock(&self);

    /// Transition from WRITE to POISON. Other states cause undefined behavior.
    unsafe fn unlock_poison(&self);

    /// Transition from WRITE TO READ. Other states cause undefined behavior.
    unsafe fn unlock_fuse(&self);

    /// Return the current state.
    fn try_get_mut(&mut self) -> Result<RawFusedState, PoisonError<()>>;
}
