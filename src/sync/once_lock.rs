use std::cell::{UnsafeCell};
use std::fmt::{Debug, Formatter};
use std::mem;
use std::mem::MaybeUninit;
use std::panic::{RefUnwindSafe, UnwindSafe};
use std::sync::atomic::Ordering::Release;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::panicking;

use crate::{OnceEntry, LockError, PoisonError};
use crate::sync::safe_mutex::{SafeMutex, SafeMutexGuard};

///
/// ```
/// use safe_once::sync::OnceLock;
/// let foo = OnceLock::<String>::new();
/// let foo_ref = foo.get_or_init(|| format!("1+1={}",1+1));
/// assert_eq!(foo_ref, "1+1=2");
/// ```
///

pub struct OnceLock<T = ()> {
    initializing: SafeMutex,
    initialized: AtomicBool,
    value: UnsafeCell<MaybeUninit<Result<T, PoisonError>>>,
}

pub struct OnceLockGuard<'a, T> {
    once: Option<&'a OnceLock<T>>,
    _inner: SafeMutexGuard<'a>,
}

impl<T> OnceLock<T> {
    pub const fn new() -> Self {
        OnceLock {
            initializing: SafeMutex::new(),
            initialized: AtomicBool::new(false),
            value: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }
    pub const fn poisoned() -> Self {
        OnceLock {
            initializing: SafeMutex::new(),
            initialized: AtomicBool::new(true),
            value: UnsafeCell::new(MaybeUninit::new(Err(PoisonError))),
        }
    }
    unsafe fn raw_get(&self) -> Result<&T, PoisonError> {
        match (*self.value.get()).assume_init_ref() {
            Ok(x) => Ok(x),
            Err(e) => Err(*e)
        }
    }
    unsafe fn raw_init(&self, value: Result<T, PoisonError>) {
        (*self.value.get()).write(value);
    }
    unsafe fn raw_take(&self) -> Result<T, PoisonError> {
        (*self.value.get()).assume_init_read()
    }
    pub fn get_checked(&self) -> Result<Option<&T>, PoisonError> {
        unsafe {
            if self.initialized.load(Ordering::Acquire) {
                Ok(Some(self.raw_get()?))
            } else {
                Ok(None)
            }
        }
    }
    pub fn lock_checked(&self) -> Result<OnceEntry<&T, OnceLockGuard<T>>, LockError> {
        if let Some(result) = self.get_checked()? {
            return Ok(OnceEntry::Occupied(result));
        }
        let lock = self.initializing.lock()?;
        if let Some(result) = self.get_checked()? {
            return Ok(OnceEntry::Occupied(result));
        }
        Ok(OnceEntry::Vacant(OnceLockGuard {
            once: Some(self),
            _inner: lock,
        }))
    }
    pub fn get_or_init_checked<F: FnOnce() -> T>(&self, init: F) -> Result<&T, LockError> {
        match self.lock_checked()? {
            OnceEntry::Occupied(x) => Ok(x),
            OnceEntry::Vacant(x) => Ok(x.init(init())),
        }
    }
    pub fn into_inner_checked(mut self) -> Result<Option<T>, PoisonError> {
        unsafe {
            if mem::replace(self.initialized.get_mut(), false) {
                Ok(Some(self.raw_take()?))
            } else {
                Ok(None)
            }
        }
    }
    pub fn get(&self) -> Option<&T> {
        self.get_checked().unwrap()
    }
    pub fn lock(&self) -> OnceEntry<&T, OnceLockGuard<T>> {
        self.lock_checked().unwrap()
    }
    pub fn get_or_init<F: FnOnce() -> T>(&self, init: F) -> &T {
        self.get_or_init_checked(init).unwrap()
    }
    pub fn into_inner(self) -> Option<T> {
        self.into_inner_checked().unwrap()
    }
}

impl<'a, T> OnceLockGuard<'a, T> {
    pub fn init(mut self, x: T) -> &'a T {
        unsafe {
            let once = self.once.take().unwrap();
            once.raw_init(Ok(x));
            once.initialized.store(true, Release);
            once.raw_get().unwrap()
        }
    }
}

impl<'a, T> Drop for OnceLockGuard<'a, T> {
    fn drop(&mut self) {
        unsafe {
            if let Some(once) = self.once {
                if panicking() {
                    once.raw_init(Err(PoisonError));
                    once.initialized.store(true, Release);
                }
            }
        }
    }
}

impl<T> From<T> for OnceLock<T> {
    fn from(x: T) -> Self {
        OnceLock {
            initializing: SafeMutex::new(),
            initialized: AtomicBool::new(true),
            value: UnsafeCell::new(MaybeUninit::new(Ok(x))),
        }
    }
}

unsafe impl<T: Send> Send for OnceLock<T> {}

unsafe impl<T: Sync + Send> Sync for OnceLock<T> {}

impl<T: RefUnwindSafe + UnwindSafe> RefUnwindSafe for OnceLock<T> {}

impl<T: UnwindSafe> UnwindSafe for OnceLock<T> {}

impl<T: Clone> Clone for OnceLock<T> {
    fn clone(&self) -> Self {
        match self.get_checked() {
            Ok(Some(x)) => OnceLock::from(x.clone()),
            Ok(None) => OnceLock::new(),
            Err(_) => OnceLock::poisoned(),
        }
    }
}

impl<T> Default for OnceLock<T> {
    fn default() -> Self { OnceLock::new() }
}

impl<T: Debug> Debug for OnceLock<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        unsafe {
            if self.initialized.load(Ordering::Acquire) {
                match self.raw_get() {
                    Ok(x) => f.debug_tuple("OnceLock::Initialized").field(&x).finish(),
                    Err(_) => write!(f, "OnceLock::Poisoned")
                }
            } else if let Some(_) = self.initializing.try_lock() {
                write!(f, "OnceLock::Uninit")
            } else {
                write!(f, "OnceLock::Initializing")
            }
        }
    }
}