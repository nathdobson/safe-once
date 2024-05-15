use crate::sync::thread_id::ThreadId;
use std::thread::Thread;

#[derive(Eq, Ord, PartialEq, PartialOrd, Copy, Clone, Debug)]
pub struct State(usize);

const INIT_BIT: usize = 0b0001;
const LOCKED_BIT: usize = 0b0010;
const PARKED_BIT: usize = 0b0100;
const POISON_BIT: usize = 0b1000;
const THREAD_ID_MASK: usize = !(0b1111);

impl State {
    pub const fn new() -> Self {
        State(0)
    }

    const fn get_bit(self, index: usize) -> bool {
        self.0 & index != 0
    }

    const fn with_bit(self, index: usize, value: bool) -> Self {
        if value {
            State(self.0 | index)
        } else {
            State(self.0 & !index)
        }
    }

    pub const fn init(self) -> bool {
        self.get_bit(INIT_BIT)
    }
    #[must_use]
    pub const fn with_init(self, value: bool) -> Self {
        self.with_bit(INIT_BIT, value)
    }

    pub const fn poison(self) -> bool {
        self.0 & POISON_BIT != 0
    }
    #[must_use]
    pub const fn with_poison(self, value: bool) -> Self {
        self.with_bit(POISON_BIT, value)
    }

    pub const fn locked(self) -> bool {
        self.0 & LOCKED_BIT != 0
    }
    #[must_use]
    pub const fn with_locked(self, value: bool) -> Self {
        self.with_bit(LOCKED_BIT, value)
    }

    pub const fn parked(self) -> bool {
        self.0 & PARKED_BIT != 0
    }
    #[must_use]
    pub const fn with_parked(self, value: bool) -> Self {
        self.with_bit(PARKED_BIT, value)
    }

    pub const fn thread_id(self) -> ThreadId {
        ThreadId(self.0 & THREAD_ID_MASK)
    }
    #[must_use]
    pub const fn with_thread_id(self, id: ThreadId) -> Self {
        State((self.0 & !THREAD_ID_MASK) | id.0)
    }
}
