use std::panic::catch_unwind;
use crate::{LockError, OnceEntry, PoisonError};
use crate::sync::{OnceLock};

#[test]
fn test_once() {
    let lock = OnceLock::<Box<usize>>::new();
    match lock.lock_checked().unwrap() {
        OnceEntry::Occupied(_) => unreachable!(),
        OnceEntry::Vacant(x) => { x.init(Box::new(1)); }
    }
    match lock.lock_checked().unwrap() {
        OnceEntry::Occupied(x) => assert_eq!(**x, 1),
        OnceEntry::Vacant(_) => unreachable!(),
    };
}

#[test]
fn test_direct() {
    assert!(OnceLock::<Box<isize>>::new().into_inner().is_none());
    assert_eq!(*OnceLock::from(Box::new(1)).into_inner().unwrap(), 1);
}

#[test]
fn test_relock() {
    let once = OnceLock::<Box<isize>>::new();
    match once.lock() {
        OnceEntry::Occupied(_) => unreachable!(),
        OnceEntry::Vacant(_) => {}
    }
    match once.lock() {
        OnceEntry::Occupied(_) => unreachable!(),
        OnceEntry::Vacant(x) => { x.init(Box::new(5)); }
    }
    assert_eq!(**once.get().unwrap(), 5);
}

#[test]
fn test_recurrent() {
    let once = OnceLock::<Box<isize>>::new();
    once.get_or_init(|| {
        assert_eq!(once.get_or_init_checked(|| unreachable!()).unwrap_err(), LockError::CycleError);
        Box::new(5)
    });
}

#[test]
fn test_panic() {
    let once = OnceLock::<Box<isize>>::new();
    assert!(catch_unwind(|| {
        once.get_or_init(|| {
            panic!();
        });
    }).is_err());
    assert_eq!(once.get_checked().unwrap_err(), PoisonError);
}