use std::cell::{Cell, UnsafeCell};
use std::fmt::{Debug, Formatter};
use std::mem::MaybeUninit;
use std::sync::PoisonError;
use std::thread::panicking;
use crate::{OnceEntry, LockError};

#[derive(Copy, Clone)]
enum State {
    Uninit,
    Initializing,
    Initialized,
    Poison,
}

pub struct OnceCell<T = ()> {
    state: Cell<State>,
    value: UnsafeCell<MaybeUninit<T>>,
}

pub struct OnceCellGuard<'a, T> {
    once: Option<&'a OnceCell<T>>,
}


///
/// ```
/// use safe_once::unsync::OnceCell;
/// let foo = OnceCell::<String>::new();
/// let foo_ref = foo.get_or_init(|| format!("1+1={}",1+1));
/// assert_eq!(foo_ref, "1+1=2");
/// ```
///

impl<T> OnceCell<T> {
    pub const fn new() -> Self {
        OnceCell {
            state: Cell::new(State::Uninit),
            value: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }
    pub const fn poisoned() -> Self {
        OnceCell {
            state: Cell::new(State::Poison),
            value: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }
    unsafe fn raw_get(&self) -> &T {
        (*self.value.get()).assume_init_ref()
    }
    unsafe fn raw_init(&self, value: T) {
        (*self.value.get()).write(value);
    }
    unsafe fn raw_take(&self) -> T {
        (*self.value.get()).assume_init_read()
    }
    pub fn get_checked(&self) -> Result<Option<&T>, PoisonError<()>> {
        unsafe {
            match self.state.get() {
                State::Initialized => Ok(Some(self.raw_get())),
                State::Poison => Err(PoisonError::new(())),
                _ => Ok(None),
            }
        }
    }
    pub fn lock_checked(&self) -> Result<OnceEntry<&T, OnceCellGuard<T>>, LockError> {
        unsafe {
            match self.state.get() {
                State::Uninit => {
                    self.state.set(State::Initializing);
                    return Ok(OnceEntry::Vacant(OnceCellGuard { once: Some(self) }));
                }
                State::Initializing => Err(LockError::CycleError),
                State::Initialized => return Ok(OnceEntry::Occupied(self.raw_get())),
                State::Poison => Err(LockError::PoisonError),
            }
        }
    }
    pub fn get_or_init_checked<F: FnOnce() -> T>(&self, init: F) -> Result<&T, LockError> {
        match self.lock_checked()? {
            OnceEntry::Vacant(guard) => Ok(guard.init(init())),
            OnceEntry::Occupied(x) => Ok(x),
        }
    }
    pub fn into_inner_checked(self) -> Result<Option<T>, PoisonError<()>> {
        unsafe {
            match self.state.replace(State::Poison) {
                State::Uninit => Ok(None),
                State::Initializing => Ok(None),
                State::Initialized => Ok(Some(self.raw_take())),
                State::Poison => Err(PoisonError::new(())),
            }
        }
    }
    pub fn get(&self) -> Option<&T> {
        self.get_checked().unwrap()
    }
    pub fn lock(&self) -> OnceEntry<&T, OnceCellGuard<T>> {
        self.lock_checked().unwrap()
    }
    #[track_caller]
    pub fn get_or_init<F: FnOnce() -> T>(&self, init: F) -> &T {
        self.get_or_init_checked(init).unwrap()
    }
    pub fn into_inner(self) -> Option<T> {
        self.into_inner_checked().unwrap()
    }
}

impl<'a, T> OnceCellGuard<'a, T> {
    pub fn init(mut self, x: T) -> &'a T {
        unsafe {
            let once = self.once.take().unwrap();
            once.raw_init(x);
            once.state.set(State::Initialized);
            once.raw_get()
        }
    }
}

impl<'a, T> Drop for OnceCellGuard<'a, T> {
    fn drop(&mut self) {
        if let Some(once) = self.once {
            if panicking() {
                once.state.set(State::Poison)
            }
        }
    }
}

impl<T> From<T> for OnceCell<T> {
    fn from(x: T) -> Self {
        OnceCell {
            state: Cell::new(State::Initialized),
            value: UnsafeCell::new(MaybeUninit::new(x)),
        }
    }
}

unsafe impl<T: Send> Send for OnceCell<T> {}

impl<T: Clone> Clone for OnceCell<T> {
    fn clone(&self) -> Self {
        unsafe {
            match self.state.get() {
                State::Uninit => OnceCell::new(),
                State::Initializing => OnceCell::new(),
                State::Initialized => OnceCell::from(self.raw_get().clone()),
                State::Poison => OnceCell::poisoned(),
            }
        }
    }
}

impl<T> Default for OnceCell<T> {
    fn default() -> Self { OnceCell::new() }
}

impl<T: Debug> Debug for OnceCell<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        unsafe {
            match self.state.get() {
                State::Uninit => write!(f, "OnceCell::Uninit"),
                State::Initializing => write!(f, "OnceCell::Initializing"),
                State::Initialized =>
                    f
                        .debug_tuple("OnceCell::Initialized")
                        .field(self.raw_get())
                        .finish(),
                State::Poison => write!(f, "OnceCell::Poisoned"),
            }
        }
    }
}