use std::cell::Cell;
use parking_lot::{ReentrantMutex, ReentrantMutexGuard};
use crate::{CycleError};

// A mutex that detects deadlocks from the same thread.
pub struct SafeMutex(ReentrantMutex<Cell<bool>>);

pub struct SafeMutexGuard<'a>(ReentrantMutexGuard<'a, Cell<bool>>);

impl SafeMutex {
    pub const fn new() -> Self { SafeMutex(ReentrantMutex::new(Cell::new(false))) }
    pub fn lock(&self) -> Result<SafeMutexGuard, CycleError> {
        let guard = self.0.lock();
        if guard.get() {
            return Err(CycleError);
        }
        guard.set(true);
        Ok(SafeMutexGuard(guard))
    }
    pub fn try_lock(&self) -> Option<SafeMutexGuard> {
        let guard = self.0.try_lock()?;
        if guard.get() {
            return None;
        }
        guard.set(true);
        Some(SafeMutexGuard(guard))
    }
}

impl<'a> Drop for SafeMutexGuard<'a> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}
