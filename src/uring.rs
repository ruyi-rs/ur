use crate::params::{IoRingSetup, IoUringBuilder};
use crate::{cq, sq, sys};

#[derive(Debug)]
pub struct Fd(i32);

impl Fd {
    #[inline]
    pub const fn new(fd: i32) -> Self {
        Self(fd)
    }

    #[inline]
    pub fn as_raw_fd(&self) -> i32 {
        self.0
    }
}

impl Drop for Fd {
    #[inline]
    fn drop(&mut self) {
        sys::close(self.0).ok();
    }
}

#[derive(Debug)]
pub struct IoUring {
    sq: sq::Queue,
    cq: cq::Queue,
    flags: IoRingSetup,
    fd: Fd,
}

impl IoUring {
    #[inline]
    pub(crate) fn new(sq: sq::Queue, cq: cq::Queue, flags: IoRingSetup, fd: Fd) -> Self {
        Self { sq, cq, flags, fd }
    }

    #[inline]
    pub const fn entries(entries: u32) -> IoUringBuilder {
        IoUringBuilder::new(entries)
    }
}
