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

impl IoUring {
    #[inline]
    pub fn entries(entries: u32) -> Builder {
        Builder::new(entries)
    }
}

impl Drop for IoUring {
    #[inline]
    fn drop(&mut self) {
        todo!()
    }
}

#[derive(Debug)]
pub struct Builder {
    entries: u32,
    builder: params::Builder,
}

impl Builder {
    #[inline]
    const fn new(entries: u32) -> Self {
        Self {
            entries,
            builder: Params::builder(),
        }
    }

    #[inline]
    pub fn try_build(&self) -> Result<IoUring> {
        let entries = self.entries.next_power_of_two();
        let mut params = self.builder.build();
        let fd = unsafe { sys::io_uring_setup(entries, params.as_mut()) };
        if fd < 0 {
            return Err(Error::last_os_error());
        }
        let mut io_uring: IoUring = unsafe { MaybeUninit::zeroed().assume_init() };
        io_uring.fd = fd;
        Ok(io_uring)
    }
}
