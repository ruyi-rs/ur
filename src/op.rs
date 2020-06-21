use std::marker::PhantomData;
use std::ptr;

use crate::sq;
use crate::Uring;

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
    fn code() -> u8;

    fn prepare<'a>(&self, uring: &'a mut Uring) -> Option<&'a mut sq::Entry>;
}

#[derive(Clone, Copy, Debug)]
pub struct Nop {
    _marker: PhantomData<()>,
}

impl Nop {
    const CODE: Code = Code::Nop;

    #[inline]
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl Op for Nop {
    #[inline]
    fn code() -> u8 {
        Self::CODE as u8
    }

    #[inline]
    fn prepare<'a>(&self, uring: &'a mut Uring) -> Option<&'a mut sq::Entry> {
        uring.sq().prep_rw(Self::CODE, -1, ptr::null(), 0, 0)
    }
}

pub struct Readv {}
// #[derive(Clone, Copy, Debug)]
// pub struct Splice {
//     fd_in: RawFd,
//     off_in: u64,
//     fd_out: RawFd,
//     off_out: u64,
//     nbytes: u32,
//     splice_flags: u32,
// }
