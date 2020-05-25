use std::io::Result;
use std::os::unix::io::RawFd;

use crate::params::{self, IoRingSetup, IoUringParams};
use crate::syscall;
use crate::sq;
use crate::cq;

#[derive(Debug)]
pub struct IoUring {
    sq: sq::Queue,
    cq: cq::Queue,
    flags: IoRingSetup,
    fd: RawFd,
}

impl IoUring {
    #[inline]
    fn new(fd: RawFd, flags: IoRingSetup) -> Self {
        Self {
            sq: sq::Queue::new(),
            cq: cq::Queue::new(),
            flags,
            fd,
        }
    }

    #[inline]
    pub fn entries(entries: u32) -> IoUringBuilder {
        IoUringBuilder::new(entries)
    }

    fn queue_mmap(&mut self, params: &IoUringParams) -> Result<()> {

        Ok(())
    }
}

impl Drop for IoUring {
    #[inline]
    fn drop(&mut self) {
        // TODO: munmap
        syscall::close(self.fd).ok();
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

    pub fn try_build(&self) -> Result<IoUring> {
        let mut params = self.params.build();
        let fd = syscall::io_uring_setup(self.entries, &mut params)?;
        let mut io_uring = IoUring::new(fd, params.flags());

        // TODO mmap

        Ok(io_uring)
    }
}
