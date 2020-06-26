use std::fmt;
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

#[derive(Debug)]
pub struct Nop;

impl Op for Nop {
    const CODE: u8 = Code::Nop as u8;

    #[inline]
    unsafe fn prepare<'a>(&self, uring: &'a mut Uring) -> Option<&'a mut sq::Entry> {
        uring.sq().prep_rw(Self::CODE, -1, ptr::null(), 0, 0)
    }
}

#[derive(Debug)]
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

#[derive(Debug)]
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

#[derive(Debug)]
pub struct Fsync {
    pub fd: RawFd,
    pub flags: u32,
}

impl Op for Fsync {
    const CODE: u8 = Code::Fsync as u8;

    #[inline]
    unsafe fn prepare<'a>(&self, uring: &'a mut Uring) -> Option<&'a mut sq::Entry> {
        match uring.sq().prep_rw(Self::CODE, self.fd, ptr::null(), 0, 0) {
            Some(sqe) => {
                sqe.set_fsync_flags(self.flags);
                Some(sqe)
            }
            None => None,
        }
    }
}

#[derive(Debug)]
pub struct ReadFixed<'a> {
    pub fd: RawFd,
    pub buf: &'a mut [u8],
    pub offset: u64,
    pub buf_index: u16,
}

impl Op for ReadFixed<'_> {
    const CODE: u8 = Code::ReadFixed as u8;

    #[inline]
    unsafe fn prepare<'a>(&self, uring: &'a mut Uring) -> Option<&'a mut sq::Entry> {
        match uring.sq().prep_rw(
            Self::CODE,
            self.fd,
            self.buf.as_ptr() as *const _,
            self.buf.len() as u32,
            self.offset,
        ) {
            Some(sqe) => {
                sqe.set_buf_index(self.buf_index);
                Some(sqe)
            }
            None => None,
        }
    }
}

#[derive(Debug)]
pub struct WriteFixed<'a> {
    pub fd: RawFd,
    pub buf: &'a [u8],
    pub offset: u64,
    pub buf_index: u16,
}

impl Op for WriteFixed<'_> {
    const CODE: u8 = Code::WriteFixed as u8;

    #[inline]
    unsafe fn prepare<'a>(&self, uring: &'a mut Uring) -> Option<&'a mut sq::Entry> {
        match uring.sq().prep_rw(
            Self::CODE,
            self.fd,
            self.buf.as_ptr() as *const _,
            self.buf.len() as u32,
            self.offset,
        ) {
            Some(sqe) => {
                sqe.set_buf_index(self.buf_index);
                Some(sqe)
            }
            None => None,
        }
    }
}

#[derive(Debug)]
pub struct PollAdd {
    pub fd: RawFd,
    pub poll_mask: u16,
}

impl Op for PollAdd {
    const CODE: u8 = Code::PollAdd as u8;

    #[inline]
    unsafe fn prepare<'a>(&self, uring: &'a mut Uring) -> Option<&'a mut sq::Entry> {
        match uring.sq().prep_rw(Self::CODE, self.fd, ptr::null(), 0, 0) {
            Some(sqe) => {
                sqe.set_poll_events(self.poll_mask);
                Some(sqe)
            }
            None => None,
        }
    }
}

#[derive(Debug)]
pub struct PollRemove {
    pub fd: RawFd,
    pub user_data: u64,
}

impl Op for PollRemove {
    const CODE: u8 = Code::PollRemove as u8;

    #[inline]
    unsafe fn prepare<'a>(&self, uring: &'a mut Uring) -> Option<&'a mut sq::Entry> {
        uring
            .sq()
            .prep_rw(Self::CODE, self.fd, self.user_data as _, 0, 0)
    }
}

#[derive(Debug)]
pub struct SyncFileRange {
    pub fd: RawFd,
    pub offset: u64,
    pub len: u32,
    pub flags: u32,
}

impl Op for SyncFileRange {
    const CODE: u8 = Code::SyncFileRange as u8;

    #[inline]
    unsafe fn prepare<'a>(&self, uring: &'a mut Uring) -> Option<&'a mut sq::Entry> {
        match uring
            .sq()
            .prep_rw(Self::CODE, self.fd, ptr::null(), self.len, self.offset)
        {
            Some(sqe) => {
                sqe.set_sync_range_flags(self.flags);
                Some(sqe)
            }
            None => None,
        }
    }
}

pub struct SendMsg<'a> {
    pub fd: RawFd,
    pub msg: &'a libc::msghdr,
    pub flags: u32,
}

impl Op for SendMsg<'_> {
    const CODE: u8 = Code::SendMsg as u8;

    #[inline]
    unsafe fn prepare<'a>(&self, uring: &'a mut Uring) -> Option<&'a mut sq::Entry> {
        match uring
            .sq()
            .prep_rw(Self::CODE, self.fd, self.msg as *const _ as *const _, 1, 0)
        {
            Some(sqe) => {
                sqe.set_msg_flags(self.flags);
                Some(sqe)
            }
            None => None,
        }
    }
}

impl fmt::Debug for SendMsg<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SendMsg {{ fd: {}, msg: msghdr {{ msg_name: {:#x}, msg_namelen: {}, msg_iov: {:#x}, msg_iovlen: {}, msg_control: {:#x}, msg_controllen: {}, msg_flags: {} }}, flags: {} }}", self.fd, self.msg.msg_name as u64, self.msg.msg_namelen, self.msg.msg_iov as u64, self.msg.msg_iovlen, self.msg.msg_control as u64, self.msg.msg_controllen, self.msg.msg_flags, self.flags)
    }
}

pub struct RecvMsg<'a> {
    pub fd: RawFd,
    pub msg: &'a mut libc::msghdr,
    pub flags: u32,
}

impl Op for RecvMsg<'_> {
    const CODE: u8 = Code::RecvMsg as u8;

    #[inline]
    unsafe fn prepare<'a>(&self, uring: &'a mut Uring) -> Option<&'a mut sq::Entry> {
        match uring
            .sq()
            .prep_rw(Self::CODE, self.fd, self.msg as *const _ as *const _, 1, 0)
        {
            Some(sqe) => {
                sqe.set_msg_flags(self.flags);
                Some(sqe)
            }
            None => None,
        }
    }
}

impl fmt::Debug for RecvMsg<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RecvMsg {{ fd: {}, msg: msghdr {{ msg_name: {:#x}, msg_namelen: {}, msg_iov: {:#x}, msg_iovlen: {}, msg_control: {:#x}, msg_controllen: {}, msg_flags: {} }}, flags: {} }}", self.fd, self.msg.msg_name as u64, self.msg.msg_namelen, self.msg.msg_iov as u64, self.msg.msg_iovlen, self.msg.msg_control as u64, self.msg.msg_controllen, self.msg.msg_flags, self.flags)
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
