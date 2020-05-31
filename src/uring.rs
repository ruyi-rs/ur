use std::fs::File;
use std::io::{IoSlice, Result};
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
pub struct Mmap<T> {
    addr: ptr::NonNull<T>,
    len: usize,
}

impl<T> Mmap<T> {
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

impl<T> Drop for Mmap<T> {
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
    // io_uring_register(2) opcodes and arguments
    const IORING_REGISTER_BUFFERS: libc::c_uint = 0;
    const IORING_UNREGISTER_BUFFERS: libc::c_uint = 1;
    const IORING_REGISTER_FILES: libc::c_uint = 2;
    const IORING_UNREGISTER_FILES: libc::c_uint = 3;
    const IORING_REGISTER_EVENTFD: libc::c_uint = 4;
    const IORING_UNREGISTER_EVENTFD: libc::c_uint = 5;
    const IORING_REGISTER_FILES_UPDATE: libc::c_uint = 6;
    const IORING_REGISTER_EVENTFD_ASYNC: libc::c_uint = 7;
    const IORING_REGISTER_PROBE: libc::c_uint = 8;
    const IORING_REGISTER_PERSONALITY: libc::c_uint = 9;
    const IORING_UNREGISTER_PERSONALITY: libc::c_uint = 10;

    #[inline]
    pub(crate) fn new(sq: sq::Queue, cq: cq::Queue, flags: Setup, fd: Fd) -> Self {
        Self { sq, cq, flags, fd }
    }

    #[inline]
    pub const fn entries(entries: u32) -> UringBuilder {
        UringBuilder::new(entries)
    }

    #[inline]
    pub fn register_buffers(&self, bufs: &[IoSlice]) -> Result<()> {
        unsafe {
            sys::io_uring_register(
                self.fd.as_raw_fd(),
                Self::IORING_REGISTER_BUFFERS,
                bufs.as_ptr() as *const _,
                bufs.len() as u32,
            )
        }
    }

    #[inline]
    pub fn unregister_buffers(&self) -> Result<()> {
        unsafe {
            sys::io_uring_register(
                self.fd.as_raw_fd(),
                Self::IORING_UNREGISTER_BUFFERS,
                ptr::null(),
                0,
            )
        }
    }

    #[inline]
    pub fn register_files(&self, files: &[&File]) -> Result<()> {
        todo!()
    }

    #[inline]
    pub fn unregister_files(&self) -> Result<()> {
        todo!()
    }
}
