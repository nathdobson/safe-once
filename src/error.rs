use std::fmt::{Display, Formatter};

#[derive(Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Hash, Debug)]
pub struct PoisonError;

#[derive(Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Hash, Debug)]
pub enum LockError {
    PoisonError,
    CycleError,
}

impl From<PoisonError> for LockError {
    fn from(_: PoisonError) -> Self {
        LockError::PoisonError
    }
}

impl Display for PoisonError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Once is poisoned")
    }
}

impl Display for LockError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            LockError::PoisonError => PoisonError.fmt(f),
            LockError::CycleError => write!(f, "Cycle while initializing Once"),
        }
    }
}
