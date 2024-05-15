#[derive(Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Debug)]
pub struct ThreadId(pub usize);

#[repr(align(16))]
struct Aligned128(#[allow(dead_code)] u128);

impl ThreadId {
    pub fn current() -> Self {
        // guarantee 4 bits of alignment by using u128
        thread_local!(static KEY: Aligned128 = Aligned128(0));
        KEY.with(|x| {
            let x = x as *const _ as usize;
            ThreadId::from(x)
        })
    }
}

impl From<usize> for ThreadId {
    fn from(x: usize) -> Self {
        assert_eq!(x & 0b1111, 0);
        assert_ne!(x, 0);
        ThreadId(x)
    }
}
