use std::io::Result;
use std::mem::MaybeUninit;
use std::os::unix::io::RawFd;

use crate::params::{self, IoUringParams};
use crate::syscall;

#[derive(Debug)]
struct SubmissionQueue {}

#[derive(Debug)]
struct CompletionQueue {}

#[derive(Debug)]
pub struct IoUring {
    sq: SubmissionQueue,
    cq: CompletionQueue,
    flags: u32,
    fd: RawFd,
}

impl IoUring {
    #[inline]
    fn new() -> Self {
        unsafe { MaybeUninit::zeroed().assume_init() }
    }

    #[inline]
    pub fn entries(entries: u32) -> IoUringBuilder {
        IoUringBuilder::new(entries)
    }
}

impl Drop for IoUring {
    #[inline]
    fn drop(&mut self) {
        todo!()
    }
}

#[derive(Debug, Copy, Clone)]
pub struct IoUringBuilder {
    entries: libc::__u32,
    params: params::Builder,
}

impl IoUringBuilder {
    #[inline]
    fn new(entries: u32) -> Self {
        Self {
            entries: entries.next_power_of_two(),
            params: IoUringParams::builder(),
        }
    }

    #[inline]
    pub fn iopoll(&mut self) -> &mut Self {
        self.params.iopoll();
        self
    }

    #[inline]
    pub fn sqpoll(&mut self) -> &mut Self {
        self.params.sqpoll();
        self
    }

    #[inline]
    pub fn sqpoll_idle(&mut self, idle: u32) -> &mut Self {
        self.params.sqpoll_idle(idle);
        self
    }

    #[inline]
    pub fn sqpoll_cpu(&mut self, cpu: u32) -> &mut Self {
        self.params.sqpoll_cpu(cpu);
        self
    }

    #[inline]
    pub fn cqsize(&mut self, cq_entries: u32) -> &mut Self {
        self.params.cqsize(cq_entries);
        self
    }

    #[inline]
    pub fn clamp(&mut self) -> &mut Self {
        self.params.clamp();
        self
    }

    #[inline]
    pub fn attach_wq(&mut self, wq_fd: RawFd) -> &mut Self {
        self.params.attach_wq(wq_fd);
        self
    }

    #[inline]
    pub fn try_build(&self) -> Result<IoUring> {
        let mut params = self.params.build();
        let fd = syscall::io_uring_setup(self.entries, &mut params)?;
        let mut io_uring = IoUring::new();
        io_uring.fd = fd;
        Ok(io_uring)
    }
}
