use std::mem::MaybeUninit;

use crate::sys;

#[derive(Debug, Copy, Clone)]
pub struct Params {
    inner: sys::IoUringParams,
}

impl Params {

    #[inline]
    pub fn new() -> Self {
        Self {
            inner: unsafe { MaybeUninit::zeroed().assume_init() },
        }
    }
}
