#![feature(default_free_fn)]
#![deny(unused_must_use)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_assignments)]


// pub mod error;
pub mod raw;

use std::cell::UnsafeCell;
use std::fmt::{Debug, Display, Formatter};
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::panic::{RefUnwindSafe, UnwindSafe};
use std::thread::panicking;
use raw::{RawOnce, RawOnceState};
// use crate::error::{LockError, PoisonError};

#[cfg(feature = "sync")]
pub mod sync;

#[cfg(feature = "unsync")]
pub mod cell;

pub mod lazy;
pub mod once;