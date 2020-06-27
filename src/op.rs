use std::ffi::CStr;
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

#[derive(Debug)]
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

#[derive(Debug)]
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

#[derive(Debug)]
pub struct Timeout<'a> {
    pub ts: &'a libc::timespec,
    pub count: u32,
    pub flags: u32,
}

impl Op for Timeout<'_> {
    const CODE: u8 = Code::Timeout as u8;

    #[inline]
    unsafe fn prepare<'a>(&self, uring: &'a mut Uring) -> Option<&'a mut sq::Entry> {
        match uring.sq().prep_rw(
            Self::CODE,
            -1,
            self.ts as *const _ as *const _,
            1,
            self.count as u64,
        ) {
            Some(sqe) => {
                sqe.set_timeout_flags(self.flags);
                Some(sqe)
            }
            None => None,
        }
    }
}

#[derive(Debug)]
pub struct TimeoutRemove {
    pub user_data: u64,
    pub flags: u32,
}

impl Op for TimeoutRemove {
    const CODE: u8 = Code::TimeoutRemove as u8;

    #[inline]
    unsafe fn prepare<'a>(&self, uring: &'a mut Uring) -> Option<&'a mut sq::Entry> {
        match uring
            .sq()
            .prep_rw(Self::CODE, -1, self.user_data as *const _, 0, 0)
        {
            Some(sqe) => {
                sqe.set_timeout_flags(self.flags);
                Some(sqe)
            }
            None => None,
        }
    }
}

#[derive(Debug)]
pub struct Accept<'a> {
    pub fd: RawFd,
    pub addr: &'a mut libc::sockaddr,
    pub addr_len: &'a mut libc::socklen_t,
    pub flags: u32,
}

impl Op for Accept<'_> {
    const CODE: u8 = Code::Accept as u8;

    #[inline]
    unsafe fn prepare<'a>(&self, uring: &'a mut Uring) -> Option<&'a mut sq::Entry> {
        match uring.sq().prep_rw(
            Self::CODE,
            self.fd,
            self.addr as *const _ as *const _,
            0,
            self.addr_len as *const _ as u64,
        ) {
            Some(sqe) => {
                sqe.set_accept_flags(self.flags);
                Some(sqe)
            }
            None => None,
        }
    }
}

#[derive(Debug)]
pub struct Cancel {
    pub user_data: u64,
    pub flags: u32,
}

impl Op for Cancel {
    const CODE: u8 = Code::AsyncCancel as u8;

    #[inline]
    unsafe fn prepare<'a>(&self, uring: &'a mut Uring) -> Option<&'a mut sq::Entry> {
        match uring
            .sq()
            .prep_rw(Self::CODE, -1, self.user_data as *const _, 0, 0)
        {
            Some(sqe) => {
                sqe.set_cancel_flags(self.flags);
                Some(sqe)
            }
            None => None,
        }
    }
}

#[derive(Clone)]
pub struct LinkTimeout<'a> {
    pub ts: &'a libc::timespec,
    pub flags: u32,
}

impl Op for LinkTimeout<'_> {
    const CODE: u8 = Code::LinkTimeout as u8;

    #[inline]
    unsafe fn prepare<'a>(&self, uring: &'a mut Uring) -> Option<&'a mut sq::Entry> {
        match uring
            .sq()
            .prep_rw(Self::CODE, -1, self.ts as *const _ as *const _, 1, 0)
        {
            Some(sqe) => {
                sqe.set_timeout_flags(self.flags);
                Some(sqe)
            }
            None => None,
        }
    }
}

#[derive(Clone)]
pub struct Connect<'a> {
    pub fd: RawFd,
    pub addr: &'a libc::sockaddr,
    pub addr_len: libc::socklen_t,
}

impl Op for Connect<'_> {
    const CODE: u8 = Code::Connect as u8;

    #[inline]
    unsafe fn prepare<'a>(&self, uring: &'a mut Uring) -> Option<&'a mut sq::Entry> {
        uring.sq().prep_rw(
            Self::CODE,
            self.fd,
            self.addr as *const _ as *const _,
            0,
            self.addr_len as u64,
        )
    }
}

#[derive(Clone)]
pub struct Fallocate {
    pub fd: RawFd,
    pub mode: u32,
    pub offset: u64,
    pub len: u64,
}

impl Op for Fallocate {
    const CODE: u8 = Code::Fallocate as u8;

    #[inline]
    unsafe fn prepare<'a>(&self, uring: &'a mut Uring) -> Option<&'a mut sq::Entry> {
        uring.sq().prep_rw(
            Self::CODE,
            self.fd,
            self.len as *const _,
            self.mode,
            self.offset,
        )
    }
}

#[derive(Clone)]
pub struct Openat<'a> {
    pub dfd: RawFd,
    pub path: &'a CStr,
    pub flags: u32,
    pub mode: u32,
}

impl Op for Openat<'_> {
    const CODE: u8 = Code::Openat as u8;

    #[inline]
    unsafe fn prepare<'a>(&self, uring: &'a mut Uring) -> Option<&'a mut sq::Entry> {
        match uring.sq().prep_rw(
            Self::CODE,
            self.dfd,
            self.path.as_ptr() as *const _,
            self.mode,
            0,
        ) {
            Some(sqe) => {
                sqe.set_open_flags(self.flags);
                Some(sqe)
            }
            None => None,
        }
    }
}

#[derive(Clone)]
pub struct Close {
    pub fd: RawFd,
}

impl Op for Close {
    const CODE: u8 = Code::Close as u8;

    #[inline]
    unsafe fn prepare<'a>(&self, uring: &'a mut Uring) -> Option<&'a mut sq::Entry> {
        uring.sq().prep_rw(Self::CODE, self.fd, ptr::null(), 0, 0)
    }
}

#[derive(Clone)]
pub struct FilesUpdate<'a> {
    pub fds: &'a [RawFd],
    pub offset: u32,
}

impl Op for FilesUpdate<'_> {
    const CODE: u8 = Code::FilesUpdate as u8;

    #[inline]
    unsafe fn prepare<'a>(&self, uring: &'a mut Uring) -> Option<&'a mut sq::Entry> {
        uring.sq().prep_rw(
            Self::CODE,
            -1,
            self.fds.as_ptr() as *const _,
            self.fds.len() as u32,
            self.offset as u64,
        )
    }
}

#[derive(Clone)]
pub struct Statx;
#[derive(Clone)]
pub struct Read;
#[derive(Clone)]
pub struct Write;
#[derive(Clone)]
pub struct Fadvise;
#[derive(Clone)]
pub struct Madvise;
#[derive(Clone)]
pub struct Send;
#[derive(Clone)]
pub struct Recv;
#[derive(Clone)]
pub struct Openat2;
#[derive(Clone)]
pub struct EpollCtl;
#[derive(Clone)]
pub struct Splice {
    pub fd_in: RawFd,
    pub off_in: u64,
    pub fd_out: RawFd,
    pub off_out: u64,
    pub nbytes: u32,
    pub flags: u32,
}
#[derive(Clone)]
pub struct ProvideBuffers;
#[derive(Clone)]
pub struct RemoveBuffers;
