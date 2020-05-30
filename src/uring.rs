use std::io::Result;
use std::ptr;

use crate::params::{Setup, UringBuilder};
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
        unsafe {
            sys::close(self.0).ok();
        }
    }
}

#[derive(Debug)]
pub struct Pointer<T> {
    addr: ptr::NonNull<T>,
    len: usize,
}

impl<T> Pointer<T> {
    #[inline]
    pub fn try_new(len: usize, fd: &Fd, offset: i64) -> Result<Self> {
        let addr = unsafe {
            let ptr = sys::mmap(
                ptr::null_mut(),
                len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED | libc::MAP_POPULATE,
                fd.as_raw_fd(),
                offset,
            )?;
            ptr::NonNull::new_unchecked(ptr as *mut T)
        };
        Ok(Self { addr, len })
    }

    #[inline]
    pub const fn as_ptr(&self) -> *mut T {
        self.addr.as_ptr()
    }
}

impl<T> Drop for Pointer<T> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            let ptr = self.addr.as_ptr() as *mut libc::c_void;
            sys::munmap(ptr, self.len).ok();
        }
    }
}

#[derive(Debug)]
pub struct Uring {
    sq: sq::Queue,
    cq: cq::Queue,
    flags: Setup,
    fd: Fd,
}

impl Uring {
    #[inline]
    pub(crate) fn new(sq: sq::Queue, cq: cq::Queue, flags: Setup, fd: Fd) -> Self {
        Self { sq, cq, flags, fd }
    }

    #[inline]
    pub const fn entries(entries: u32) -> UringBuilder {
        UringBuilder::new(entries)
    }
}
