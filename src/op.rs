use std::io::{IoSlice, IoSliceMut};
use std::os::unix::io::RawFd;
use std::ptr;

use crate::sq;
use crate::Uring;

#[repr(u8)]
#[derive(Debug, Copy, Clone)]
pub(crate) enum Code {
    Nop,
    Readv,
    Writev,
    Fsync,
    ReadFixed,
    WriteFixed,
    PollAdd,
    PollRemove,
    SyncFileRange,
    SendMsg,
    RecvMsg,
    Timeout,
    TimeoutRemove,
    Accept,
    AsyncCancel,
    LinkTimeout,
    Connect,
    Fallocate,
    Openat,
    Close,
    FilesUpdate,
    Statx,
    Read,
    Write,
    Fadvise,
    Madvise,
    Send,
    Recv,
    Openat2,
    EpollCtl,
    Splice,
    ProvideBuffers,
    RemoveBuffers,
}

pub trait Op {
    const CODE: u8;

    unsafe fn prepare<'a>(&self, uring: &'a mut Uring) -> Option<&'a mut sq::Entry>;
}

#[derive(Debug, Clone, Copy)]
pub struct Nop;

impl Op for Nop {
    const CODE: u8 = Code::Nop as u8;

    #[inline]
    unsafe fn prepare<'a>(&self, uring: &'a mut Uring) -> Option<&'a mut sq::Entry> {
        uring.sq().prep_rw(Self::CODE, -1, ptr::null(), 0, 0)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Readv<'a> {
    pub fd: RawFd,
    pub iovecs: &'a [IoSliceMut<'a>],
    pub offset: u64,
}

impl Op for Readv<'_> {
    const CODE: u8 = Code::Readv as u8;

    #[inline]
    unsafe fn prepare<'a>(&self, uring: &'a mut Uring) -> Option<&'a mut sq::Entry> {
        uring.sq().prep_rw(
            Self::CODE,
            self.fd,
            self.iovecs.as_ptr() as *const _,
            self.iovecs.len() as u32,
            self.offset,
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Writev<'a> {
    pub fd: RawFd,
    pub iovecs: &'a [IoSlice<'a>],
    pub offset: u64,
}

impl Op for Writev<'_> {
    const CODE: u8 = Code::Writev as u8;

    #[inline]
    unsafe fn prepare<'a>(&self, uring: &'a mut Uring) -> Option<&'a mut sq::Entry> {
        uring.sq().prep_rw(
            Self::CODE,
            self.fd,
            self.iovecs.as_ptr() as *const _,
            self.iovecs.len() as u32,
            self.offset,
        )
    }
}

// #[derive(Clone, Copy, Debug)]
// pub struct Splice {
//     fd_in: RawFd,
//     off_in: u64,
//     fd_out: RawFd,
//     off_out: u64,
//     nbytes: u32,
//     splice_flags: u32,
// }
