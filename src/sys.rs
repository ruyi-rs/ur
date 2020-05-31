use std::io::{Error, Result};
use std::mem;
use std::os::unix::io::RawFd;
use std::ptr;

use libc;

use crate::params::UringParams;

#[allow(non_upper_case_globals)]
const __NR_io_uring_setup: libc::c_long = 425;

#[allow(non_upper_case_globals)]
const __NR_io_uring_enter: libc::c_long = 426;

#[allow(non_upper_case_globals)]
const __NR_io_uring_register: libc::c_long = 427;

#[inline]
fn cvt(ret: i32) -> Result<i32> {
    if ret >= 0 {
        Ok(ret)
    } else {
        Err(Error::last_os_error())
    }
}

// int io_uring_setup(u32 entries, struct io_uring_params *p);
#[inline]
pub(crate) unsafe fn io_uring_setup(entries: u32, params: &mut UringParams) -> Result<RawFd> {
    let ret = libc::syscall(
        __NR_io_uring_setup,
        entries as libc::c_long,
        params as *mut UringParams as libc::c_long,
    ) as i32;
    cvt(ret)
}

// int io_uring_register(unsigned int fd, unsigned int opcode, void *arg, unsigned int nr_args);
#[inline]
pub unsafe fn io_uring_register(
    fd: RawFd,
    opcode: u32,
    arg: *const u8,
    nr_args: u32,
) -> Result<()> {
    let ret = libc::syscall(
        __NR_io_uring_register,
        fd as libc::c_long,
        opcode as libc::c_long,
        arg as libc::c_long,
        nr_args as libc::c_long,
    ) as libc::c_int;
    cvt(ret).map(drop)
}

// int io_uring_enter(unsigned int fd, unsigned int to_submit, unsigned int min_complete, unsigned int flags, sigset_t *sig);
#[inline]
pub unsafe fn io_uring_enter(
    fd: RawFd,
    to_submit: u32,
    min_complete: u32,
    flags: u32,
) -> Result<usize> {
    let n = libc::syscall(
        __NR_io_uring_enter,
        fd as libc::c_long,
        to_submit as libc::c_long,
        min_complete as libc::c_long,
        flags as libc::c_long,
        ptr::null::<libc::sigset_t>() as libc::c_long,
        0 as libc::c_long,
    ) as i32;
    cvt(n).and(Ok(n as usize))
}

#[inline]
pub unsafe fn io_uring_penter(
    fd: RawFd,
    to_submit: u32,
    min_complete: u32,
    flags: u32,
    sig: &libc::sigset_t,
) -> Result<usize> {
    let n = libc::syscall(
        __NR_io_uring_enter,
        fd as libc::c_long,
        to_submit as libc::c_long,
        min_complete as libc::c_long,
        flags as libc::c_long,
        sig as *const libc::sigset_t as libc::c_long,
        mem::size_of::<libc::sigset_t>() as libc::c_long,
    ) as i32;
    cvt(n).and(Ok(n as usize))
}

#[inline]
pub unsafe fn close(fd: RawFd) -> Result<()> {
    let ret = libc::close(fd);
    cvt(ret).and(Ok(()))
}

#[inline]
pub unsafe fn mmap(
    addr: *mut libc::c_void,
    len: usize,
    prot: i32,
    flags: i32,
    fd: RawFd,
    offset: i64,
) -> Result<*mut libc::c_void> {
    let ptr = libc::mmap(addr, len, prot, flags, fd, offset);
    if ptr != libc::MAP_FAILED {
        Ok(ptr)
    } else {
        Err(Error::last_os_error())
    }
}

#[inline]
pub unsafe fn munmap(addr: *mut libc::c_void, len: usize) -> Result<()> {
    let ret = libc::munmap(addr, len);
    cvt(ret).and(Ok(()))
}
