use std::mem;
use std::io::{Error, IoSlice, Result};
use std::os::unix::io::RawFd;
use std::ptr;

use libc;

use crate::params;

#[allow(non_upper_case_globals)]
const __NR_io_uring_register: libc::c_long = 425;

#[allow(non_upper_case_globals)]
const __NR_io_uring_setup: libc::c_long = 426;

#[allow(non_upper_case_globals)]
const __NR_io_uring_enter: libc::c_long = 427;

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

// int __sys_io_uring_register(int fd, unsigned opcode, const void *arg, unsigned nr_args)
#[inline]
unsafe fn io_uring_register(
    fd: libc::c_int,
    opcode: libc::c_uint,
    arg: *const libc::c_void,
    nr_args: libc::c_uint,
) -> Result<()> {
    let ret = libc::syscall(__NR_io_uring_register, fd, opcode, arg, nr_args);
    if ret == 0 {
        Ok(())
    } else {
        Err(Error::last_os_error())
    }
}

// int __sys_io_uring_setup(unsigned entries, struct io_uring_params *p)
#[inline]
pub fn io_uring_setup(entries: libc::c_uint, p: &mut params::IoUringParams) -> Result<RawFd> {
    let fd = unsafe { libc::syscall(__NR_io_uring_setup, entries, p) };
    if fd >= 0 {
        Ok(fd as RawFd)
    } else {
        Err(Error::last_os_error())
    }
}

#[inline]
pub fn io_uring_register_buffers(fd: RawFd, bufs: &[IoSlice]) -> Result<()> {
    unsafe {
        io_uring_register(
            fd,
            IORING_REGISTER_BUFFERS,
            bufs.as_ptr() as *const libc::c_void,
            bufs.len() as libc::c_uint,
        )
    }
}

#[inline]
pub fn io_uring_unregister_buffers(fd: RawFd) -> Result<()> {
    unsafe { io_uring_register(fd, IORING_UNREGISTER_BUFFERS, ptr::null(), 0) }
}

#[inline]
pub fn io_uring_enter(fd: RawFd, to_submit: u32, min_complete: u32, flags: u32) -> Result<usize> {
    let n = unsafe {
        libc::syscall(
            __NR_io_uring_enter,
            fd,
            to_submit,
            min_complete,
            flags,
            ptr::null::<libc::sigset_t>(),
            0,
        )
    };
    if n >= 0 {
        Ok(n as usize)
    } else {
        Err(Error::last_os_error())
    }
}

#[inline]
pub fn io_uring_penter(fd: RawFd, to_submit: u32, min_complete: u32, flags: u32, sig: &libc::sigset_t) -> Result<usize> {
    let n = unsafe {
        libc::syscall(
            __NR_io_uring_enter,
            fd,
            to_submit,
            min_complete,
            flags,
            sig,
            mem::size_of::<libc::sigset_t>(),
        )
    };
    if n >= 0 {
        Ok(n as usize)
    } else {
        Err(Error::last_os_error())
    }
}