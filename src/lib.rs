mod params;
mod sys;

use std::io::{Error, Result};
use std::mem::MaybeUninit;

use libc;

use params::Params;

#[derive(Debug)]
struct SubmissionQueue {}

#[derive(Debug)]
struct CompletionQueue {}

#[derive(Debug)]
pub struct IoUring {
    sq: SubmissionQueue,
    cq: CompletionQueue,
    flags: u32,
    fd: libc::c_int,
}

impl IoUring {}

impl Drop for IoUring {
    #[inline]
    fn drop(&mut self) {
        todo!()
    }
}

#[derive(Debug, Default)]
pub struct IoUringBuilder {
    entries: usize,
    flags: u32,
    sq_thread_cpu: u32,
    sq_thread_idle: u32,
}

impl IoUringBuilder {
    #[inline]
    pub fn new() -> Self {
        Self {
            entries: 1,
            ..Default::default()
        }
    }

    #[inline]
    pub fn entries(&mut self, entries: usize) -> &mut Self {
        assert!(
            entries >= 1 && entries <= 4096,
            "entries={} is not in the range of [1..4096]",
            entries
        );

        self.entries = entries.next_power_of_two();
        self
    }

    #[inline]
    pub fn 
    #[inline]
    pub fn try_build(&self) -> Result<IoUring> {
        let fd = unsafe { sys::io_uring_setup(self.entries as libc::c_uint, &mut params) };
        if fd < 0 {
            return Err(Error::last_os_error());
        }
        let mut io_uring: IoUring = unsafe { MaybeUninit::zeroed().assume_init() };
        io_uring.fd = fd;
        Ok(io_uring)
    }
}
