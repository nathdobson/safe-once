#![deny(unused_must_use)]

use std::fmt::{Display, Formatter};

#[cfg(feature = "sync")]
pub mod sync;

#[cfg(feature = "unsync")]
pub mod unsync;

#[derive(Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Hash, Debug)]
pub struct CycleError;

#[derive(Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Hash, Debug)]
pub struct PoisonError;

#[derive(Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Hash, Debug)]
pub enum LockError {
    PoisonError,
    CycleError,
}

impl From<CycleError> for LockError {
    fn from(_: CycleError) -> Self {
        LockError::CycleError
    }
}

impl From<PoisonError> for LockError {
    fn from(_: PoisonError) -> Self {
        LockError::PoisonError
    }
}

pub enum OnceEntry<O, V> {
    Occupied(O),
    Vacant(V),
}

impl Display for PoisonError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "poisoned")
    }
}

impl Display for CycleError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "cycle")
    }
}

impl Display for LockError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            LockError::PoisonError => PoisonError.fmt(f),
            LockError::CycleError => CycleError.fmt(f),
        }
    }
}