use std::mem::MaybeUninit;

use libc;
use crate::sys;

#[derive(Debug, Copy, Clone)]
pub struct Params {
    inner: sys::IoUringParams,
}

impl Params {
    #[inline]
    pub const fn builder() -> Builder {
        Builder::new()
    }
}

impl AsMut<sys::IoUringParams> for Params {
    #[inline]
    fn as_mut(&mut self) -> &mut sys::IoUringParams {
        &mut self.inner
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Builder {
    flags: libc::__u32, // IORING_SETUP_ flags (IoRingSetup::*)
    sq_thread_cpu: libc::__u32,
    sq_thread_idle: libc::__u32,
}

impl Builder {

    #[inline]
    const fn new() -> Self {
        Self {
            flags: 0,
            sq_thread_cpu: 0,
            sq_thread_idle: 0,
        }
    }

    #[inline]
    pub fn build(&self) -> Params {
        let mut inner: sys::IoUringParams = unsafe { MaybeUninit::zeroed().assume_init() };
        inner.flags = self.flags;
        inner.sq_thread_cpu = self.sq_thread_cpu;
        inner.sq_thread_idle = self.sq_thread_idle;
        Params { inner }
    }
}