use std::ffi::CStr;
use std::io::{IoSlice, IoSliceMut};
use std::mem;
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

#[derive(Debug)]
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

#[derive(Debug)]
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

#[derive(Debug)]
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

#[derive(Debug)]
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

#[derive(Debug)]
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

#[derive(Debug)]
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

#[derive(Debug)]
pub struct Statx<'a> {
    pub dfd: RawFd,
    pub path: &'a CStr,
    pub flags: u32,
    pub mask: u32,
    pub statxbuf: &'a libc::statx,
}

impl Op for Statx<'_> {
    const CODE: u8 = Code::Statx as u8;

    #[inline]
    unsafe fn prepare<'a>(&self, uring: &'a mut Uring) -> Option<&'a mut sq::Entry> {
        match uring.sq().prep_rw(
            Self::CODE,
            self.dfd,
            self.path.as_ptr() as *const _,
            self.mask,
            self.statxbuf as *const _ as u64,
        ) {
            Some(sqe) => {
                sqe.set_statx_flags(self.flags);
                Some(sqe)
            }
            None => None,
        }
    }
}

#[derive(Debug)]
pub struct Read<'a> {
    pub fd: RawFd,
    pub buf: &'a mut [u8],
    pub offset: u64,
}

impl Op for Read<'_> {
    const CODE: u8 = Code::Read as u8;

    #[inline]
    unsafe fn prepare<'a>(&self, uring: &'a mut Uring) -> Option<&'a mut sq::Entry> {
        uring.sq().prep_rw(
            Self::CODE,
            self.fd,
            self.buf.as_ptr() as *const _,
            self.buf.len() as u32,
            self.offset,
        )
    }
}

#[derive(Debug)]
pub struct Write<'a> {
    pub fd: RawFd,
    pub data: &'a [u8],
    pub offset: u64,
}

impl Op for Write<'_> {
    const CODE: u8 = Code::Write as u8;

    #[inline]
    unsafe fn prepare<'a>(&self, uring: &'a mut Uring) -> Option<&'a mut sq::Entry> {
        uring.sq().prep_rw(
            Self::CODE,
            self.fd,
            self.data.as_ptr() as *const _,
            self.data.len() as u32,
            self.offset,
        )
    }
}

#[derive(Debug)]
pub struct Fadvise {
    pub fd: RawFd,
    pub offset: u64,
    pub len: u32,
    pub advice: i32,
}

impl Op for Fadvise {
    const CODE: u8 = Code::Fadvise as u8;

    #[inline]
    unsafe fn prepare<'a>(&self, uring: &'a mut Uring) -> Option<&'a mut sq::Entry> {
        match uring
            .sq()
            .prep_rw(Self::CODE, self.fd, ptr::null(), self.len, self.offset)
        {
            Some(sqe) => {
                sqe.set_fadvise_advice_flags(self.advice as u32);
                Some(sqe)
            }
            None => None,
        }
    }
}

#[derive(Debug)]
pub struct Madvise<'a> {
    pub mem: &'a [u8],
    pub advice: i32,
}

impl Op for Madvise<'_> {
    const CODE: u8 = Code::Madvise as u8;

    #[inline]
    unsafe fn prepare<'a>(&self, uring: &'a mut Uring) -> Option<&'a mut sq::Entry> {
        match uring.sq().prep_rw(
            Self::CODE,
            -1,
            self.mem.as_ptr() as *const _,
            self.mem.len() as u32,
            0,
        ) {
            Some(sqe) => {
                sqe.set_fadvise_advice_flags(self.advice as u32);
                Some(sqe)
            }
            None => None,
        }
    }
}

#[derive(Debug)]
pub struct Send<'a> {
    pub sockfd: RawFd,
    pub data: &'a [u8],
    pub flags: u32,
}

impl Op for Send<'_> {
    const CODE: u8 = Code::Send as u8;

    #[inline]
    unsafe fn prepare<'a>(&self, uring: &'a mut Uring) -> Option<&'a mut sq::Entry> {
        match uring.sq().prep_rw(
            Self::CODE,
            self.sockfd,
            self.data.as_ptr() as *const _,
            self.data.len() as u32,
            0,
        ) {
            Some(sqe) => {
                sqe.set_msg_flags(self.flags);
                Some(sqe)
            }
            None => None,
        }
    }
}
#[derive(Debug)]
pub struct Recv<'a> {
    pub sockfd: RawFd,
    pub buf: &'a mut [u8],
    pub flags: u32,
}

impl Op for Recv<'_> {
    const CODE: u8 = Code::Recv as u8;

    #[inline]
    unsafe fn prepare<'a>(&self, uring: &'a mut Uring) -> Option<&'a mut sq::Entry> {
        match uring.sq().prep_rw(
            Self::CODE,
            self.sockfd,
            self.buf.as_ptr() as *const _,
            self.buf.len() as u32,
            0,
        ) {
            Some(sqe) => {
                sqe.set_msg_flags(self.flags);
                Some(sqe)
            }
            None => None,
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct OpenHow {
    pub flags: u64,
    pub mode: u64,
    pub resolve: u64,
}

#[derive(Debug)]
pub struct Openat2<'a> {
    pub dfd: RawFd,
    pub path: &'a CStr,
    pub how: &'a OpenHow,
}

impl Op for Openat2<'_> {
    const CODE: u8 = Code::Openat2 as u8;

    #[inline]
    unsafe fn prepare<'a>(&self, uring: &'a mut Uring) -> Option<&'a mut sq::Entry> {
        uring.sq().prep_rw(
            Self::CODE,
            self.dfd,
            self.path.as_ptr() as *const _,
            mem::size_of::<OpenHow>() as u32,
            self.how as *const _ as u64,
        )
    }
}

#[derive(Debug)]
pub struct EpollCtl<'a> {
    pub epfd: RawFd,
    pub fd: RawFd,
    pub op: u32,
    pub ev: &'a libc::epoll_event,
}

impl Op for EpollCtl<'_> {
    const CODE: u8 = Code::EpollCtl as u8;

    #[inline]
    unsafe fn prepare<'a>(&self, uring: &'a mut Uring) -> Option<&'a mut sq::Entry> {
        uring.sq().prep_rw(
            Self::CODE,
            self.epfd,
            self.ev as *const _ as *const _,
            self.op,
            self.fd as u64,
        )
    }
}

#[derive(Debug)]
pub struct Splice {
    pub fd_in: RawFd,
    pub off_in: u64,
    pub fd_out: RawFd,
    pub off_out: u64,
    pub nbytes: u32,
    pub flags: u32,
}

impl Op for Splice {
    const CODE: u8 = Code::Splice as u8;

    #[inline]
    unsafe fn prepare<'a>(&self, uring: &'a mut Uring) -> Option<&'a mut sq::Entry> {
        match uring.sq().prep_rw(
            Self::CODE,
            self.fd_out,
            ptr::null(),
            self.nbytes,
            self.off_out,
        ) {
            Some(sqe) => {
                sqe.set_splice_off_in(self.off_in);
                sqe.set_splice_fd_in(self.fd_in);
                sqe.set_splice_flags(self.flags);
                Some(sqe)
            }
            None => None,
        }
    }
}

#[derive(Debug)]
pub struct ProvideBuffers<'a> {
    pub addr: &'a [u8],
    pub nr: i32,
    pub bgid: u16,
    pub bid: u32,
}

impl Op for ProvideBuffers<'_> {
    const CODE: u8 = Code::ProvideBuffers as u8;

    #[inline]
    unsafe fn prepare<'a>(&self, uring: &'a mut Uring) -> Option<&'a mut sq::Entry> {
        match uring.sq().prep_rw(
            Self::CODE,
            self.nr,
            self.addr.as_ptr() as *const _,
            self.addr.len() as u32,
            self.bid as u64,
        ) {
            Some(sqe) => {
                sqe.set_buf_group(self.bgid);
                Some(sqe)
            }
            None => None,
        }
    }
}

#[derive(Debug)]
pub struct RemoveBuffers {
    pub nr: i32,
    pub bgid: u16,
}

impl Op for RemoveBuffers {
    const CODE: u8 = Code::RemoveBuffers as u8;

    #[inline]
    unsafe fn prepare<'a>(&self, uring: &'a mut Uring) -> Option<&'a mut sq::Entry> {
        match uring.sq().prep_rw(Self::CODE, self.nr, ptr::null(), 0, 0) {
            Some(sqe) => {
                sqe.set_buf_group(self.bgid);
                Some(sqe)
            }
            None => None,
        }
    }
}
