use std::alloc::{alloc_zeroed, Layout};
use std::ffi::CStr;
use std::fmt;
use std::io::{IoSlice, IoSliceMut, Result};
use std::mem;
use std::ops::{Deref, DerefMut};
use std::os::unix::io::{AsRawFd, RawFd};
use std::ptr;

use crate::params::{Setup, UringBuilder};
use crate::{cq, sq, sys};

#[derive(Debug, Copy, Clone)]
pub enum Op {
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

impl Op {
    const SUPPORTED: u16 = 1 << 0;

    pub fn is_supported(self, probe: &Probe) -> bool {
        let opcode = self as u8;
        if opcode <= probe.last_op {
            let probe_op = unsafe { probe.ops.get_unchecked(opcode as usize) };
            probe_op.flags & Self::SUPPORTED != 0
        } else {
            false
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct OpenHow {
    flags: u64,
    mode: u64,
    resolve: u64,
}

#[derive(Debug)]
pub(crate) struct Fd(RawFd);

impl Fd {
    #[inline]
    pub const fn new(fd: RawFd) -> Self {
        Self(fd)
    }
}

impl AsRawFd for Fd {
    #[inline]
    fn as_raw_fd(&self) -> i32 {
        self.0
    }
}

impl Drop for Fd {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            sys::close(self.0).ok();
        }
    }
}

#[derive(Debug)]
pub(crate) struct Mmap<T> {
    addr: ptr::NonNull<T>,
    len: usize,
}

impl<T> Mmap<T> {
    #[inline]
    pub fn try_new(len: usize, fd: &Fd, offset: i64) -> Result<Self> {
        let addr = unsafe {
            let ptr = sys::mmap(
                ptr::null_mut(),
                len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED | libc::MAP_POPULATE,
                fd.as_raw_fd(),
                offset,
            )?;
            ptr::NonNull::new_unchecked(ptr as *mut T)
        };
        Ok(Self { addr, len })
    }

    #[inline]
    pub const fn as_mut_ptr(&self) -> *mut T {
        self.addr.as_ptr()
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ProbeOp {
    op: u8,
    _resv: u8,
    flags: u16, // IO_URING_OP_* flags
    _resv2: u32,
}

impl fmt::Debug for ProbeOp {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ProbeOp {{ op: {}, flags: {:#06x} }}",
            self.op, self.flags
        )
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Probe {
    last_op: u8, // last opcode supported
    ops_len: u8, // length of ops[] array below
    _resv: u16,
    _resv2: [u32; 3],
    ops: [ProbeOp; 256],
}

impl fmt::Debug for Probe {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Probe {{ last_op: {}, ops_len: {} }}",
            self.last_op, self.ops_len
        )
    }
}

#[derive(Debug)]
pub struct Uring {
    sq: sq::Queue,
    cq: cq::Queue,
    flags: Setup,
    fd: Fd,
}

impl Uring {
    // io_uring_register(2) opcodes and arguments
    const REGISTER_BUFFERS: libc::c_uint = 0;
    const UNREGISTER_BUFFERS: libc::c_uint = 1;
    const REGISTER_FILES: libc::c_uint = 2;
    const UNREGISTER_FILES: libc::c_uint = 3;
    const REGISTER_EVENTFD: libc::c_uint = 4;
    const UNREGISTER_EVENTFD: libc::c_uint = 5;
    const REGISTER_FILES_UPDATE: libc::c_uint = 6;
    const REGISTER_EVENTFD_ASYNC: libc::c_uint = 7;
    const REGISTER_PROBE: libc::c_uint = 8;
    const REGISTER_PERSONALITY: libc::c_uint = 9;
    const UNREGISTER_PERSONALITY: libc::c_uint = 10;

    #[inline]
    pub(crate) fn new(sq: sq::Queue, cq: cq::Queue, flags: Setup, fd: Fd) -> Self {
        Self { sq, cq, flags, fd }
    }

    #[inline]
    pub const fn entries(entries: u32) -> UringBuilder {
        UringBuilder::new(entries)
    }

    #[inline]
    pub unsafe fn register_buffers(&self, bufs: &[IoSliceMut]) -> Result<()> {
        sys::io_uring_register(
            self.fd.as_raw_fd(),
            Self::REGISTER_BUFFERS,
            bufs.as_ptr() as *const _,
            bufs.len() as u32,
        )
    }

    #[inline]
    pub unsafe fn unregister_buffers(&self) -> Result<()> {
        sys::io_uring_register(
            self.fd.as_raw_fd(),
            Self::UNREGISTER_BUFFERS,
            ptr::null(),
            0,
        )
    }

    #[inline]
    pub unsafe fn register_files(&self, fds: &[RawFd]) -> Result<()> {
        sys::io_uring_register(
            self.fd.as_raw_fd(),
            Self::REGISTER_FILES,
            fds.as_ptr() as *const _,
            fds.len() as u32,
        )
    }

    #[inline]
    pub unsafe fn unregister_files(&self) -> Result<()> {
        sys::io_uring_register(self.fd.as_raw_fd(), Self::UNREGISTER_FILES, ptr::null(), 0)
    }

    #[inline]
    pub unsafe fn register_files_update(&self, offset: u32, fds: &[RawFd]) -> Result<()> {
        // io_uring_files_update
        #[repr(C, align(8))]
        #[derive(Debug, Default, Copy, Clone)]
        struct FilesUpdate {
            offset: u32,
            _resv: u32,
            fds: u64,
        }

        let fu = FilesUpdate {
            offset,
            fds: fds.as_ptr() as _,
            ..Default::default()
        };

        sys::io_uring_register(
            self.fd.as_raw_fd(),
            Self::REGISTER_FILES_UPDATE,
            &fu as *const _ as *const _,
            fds.len() as u32,
        )
    }

    #[inline]
    pub unsafe fn register_eventfd(&self, event_fd: RawFd) -> Result<()> {
        sys::io_uring_register(
            self.fd.as_raw_fd(),
            Self::REGISTER_EVENTFD,
            &event_fd as *const _ as *const _,
            1,
        )
    }

    #[inline]
    pub unsafe fn unregister_eventfd(&self) -> Result<()> {
        sys::io_uring_register(
            self.fd.as_raw_fd(),
            Self::UNREGISTER_EVENTFD,
            ptr::null(),
            0,
        )
    }

    #[inline]
    pub unsafe fn register_eventfd_async(&self, event_fd: RawFd) -> Result<()> {
        sys::io_uring_register(
            self.fd.as_raw_fd(),
            Self::REGISTER_EVENTFD_ASYNC,
            &event_fd as *const _ as *const _,
            1,
        )
    }

    #[inline]
    pub unsafe fn register_personality(&self) -> Result<()> {
        sys::io_uring_register(
            self.fd.as_raw_fd(),
            Self::REGISTER_PERSONALITY,
            ptr::null(),
            0,
        )
    }

    #[inline]
    pub unsafe fn unregister_personality(&self, id: i32) -> Result<()> {
        sys::io_uring_register(
            self.fd.as_raw_fd(),
            Self::UNREGISTER_PERSONALITY,
            ptr::null(),
            id as u32,
        )
    }

    pub fn probe(&self) -> Result<Box<Probe>> {
        let layout = Layout::new::<Probe>();
        let probe;
        unsafe {
            let ptr = alloc_zeroed(layout);
            probe = Box::from_raw(ptr as *mut Probe);
            sys::io_uring_register(self.fd.as_raw_fd(), Self::REGISTER_PROBE, ptr, 256)?;
        }
        Ok(probe)
    }

    #[inline]
    pub unsafe fn prep_splice(
        &mut self,
        fd_in: RawFd,
        off_in: u64,
        fd_out: RawFd,
        off_out: u64,
        nbytes: u32,
        splice_flags: u32,
    ) -> bool {
        match self
            .sq
            .prep_rw(Op::Splice, fd_out, ptr::null(), nbytes, off_out)
        {
            Some(sqe) => {
                sqe.set_splice_off_in(off_in);
                sqe.set_splice_fd_in(fd_in);
                sqe.set_splice_flags(splice_flags);
                true
            }
            None => false,
        }
    }

    #[inline]
    pub unsafe fn prep_readv(&mut self, fd: RawFd, iovecs: &[IoSliceMut], offset: u64) -> bool {
        self.sq
            .prep_rw(
                Op::Readv,
                fd,
                iovecs.as_ptr() as *const _,
                iovecs.len() as u32,
                offset,
            )
            .is_some()
    }

    #[inline]
    pub unsafe fn prep_read_fixed(
        &mut self,
        fd: RawFd,
        buf: &mut [u8],
        offset: u64,
        buf_index: u16,
    ) -> bool {
        match self.sq.prep_rw(
            Op::ReadFixed,
            fd,
            buf.as_ptr() as *const _,
            buf.len() as u32,
            offset,
        ) {
            Some(sqe) => {
                sqe.set_buf_index(buf_index);
                true
            }
            None => false,
        }
    }

    #[inline]
    pub unsafe fn prep_writev(&mut self, fd: RawFd, iovecs: &[IoSlice], offset: u64) -> bool {
        self.sq
            .prep_rw(
                Op::Writev,
                fd,
                iovecs.as_ptr() as *const _,
                iovecs.len() as u32,
                offset,
            )
            .is_some()
    }

    #[inline]
    pub unsafe fn prep_write_fixed(
        &mut self,
        fd: RawFd,
        buf: &[u8],
        offset: u64,
        buf_index: u16,
    ) -> bool {
        match self.sq.prep_rw(
            Op::WriteFixed,
            fd,
            buf.as_ptr() as *const _,
            buf.len() as u32,
            offset,
        ) {
            Some(sqe) => {
                sqe.set_buf_index(buf_index);
                true
            }
            None => false,
        }
    }

    #[inline]
    pub unsafe fn prep_recvmsg(&mut self, fd: RawFd, msg: &mut libc::msghdr, flags: u32) -> bool {
        match self
            .sq
            .prep_rw(Op::RecvMsg, fd, msg as *const _ as *const _, 1, 0)
        {
            Some(sqe) => {
                sqe.set_msg_flags(flags);
                true
            }
            None => false,
        }
    }

    #[inline]
    pub unsafe fn prep_sendmsg(&mut self, fd: RawFd, msg: &libc::msghdr, flags: u32) -> bool {
        match self
            .sq
            .prep_rw(Op::SendMsg, fd, msg as *const _ as *const _, 1, 0)
        {
            Some(sqe) => {
                sqe.set_msg_flags(flags);
                true
            }
            None => false,
        }
    }

    #[inline]
    pub unsafe fn prep_poll_add(&mut self, fd: RawFd, poll_mask: u16) -> bool {
        match self.sq.prep_rw(Op::PollAdd, fd, ptr::null(), 0, 0) {
            Some(sqe) => {
                sqe.set_poll_events(poll_mask);
                true
            }
            None => false,
        }
    }

    #[inline]
    pub unsafe fn prep_poll_remove(&mut self, fd: RawFd, user_data: *const libc::c_void) -> bool {
        self.sq
            .prep_rw(Op::PollRemove, fd, user_data, 0, 0)
            .is_some()
    }

    #[inline]
    pub unsafe fn prep_fsync(&mut self, fd: RawFd, fsync_flags: u32) -> bool {
        match self.sq.prep_rw(Op::Fsync, fd, ptr::null(), 0, 0) {
            Some(sqe) => {
                sqe.set_fsync_flags(fsync_flags);
                true
            }
            None => false,
        }
    }

    #[inline]
    pub unsafe fn prep_nop(&mut self) -> bool {
        self.sq.prep_rw(Op::Nop, -1, ptr::null(), 0, 0).is_some()
    }

    #[inline]
    pub unsafe fn prep_timeout(&mut self, ts: &libc::timespec, count: u32, flags: u32) -> bool {
        match self
            .sq
            .prep_rw(Op::Timeout, -1, ts as *const _ as *const _, 1, count as u64)
        {
            Some(sqe) => {
                sqe.set_timeout_flags(flags);
                true
            }
            None => false,
        }
    }

    #[inline]
    pub unsafe fn prep_timeout_remove(&mut self, user_data: u64, flags: u32) -> bool {
        match self
            .sq
            .prep_rw(Op::TimeoutRemove, -1, user_data as *const _, 0, 0)
        {
            Some(sqe) => {
                sqe.set_timeout_flags(flags);
                true
            }
            None => false,
        }
    }

    #[inline]
    pub unsafe fn prep_accept(
        &mut self,
        fd: RawFd,
        addr: &mut libc::sockaddr,
        addr_len: &mut libc::socklen_t,
        flags: u32,
    ) -> bool {
        match self.sq.prep_rw(
            Op::Accept,
            fd,
            addr as *const _ as *const _,
            0,
            addr_len as *const _ as u64,
        ) {
            Some(sqe) => {
                sqe.set_accept_flags(flags);
                true
            }
            None => false,
        }
    }

    #[inline]
    pub unsafe fn prep_cancel(&mut self, user_data: *const libc::c_void, flags: u32) -> bool {
        match self.sq.prep_rw(Op::AsyncCancel, -1, user_data, 0, 0) {
            Some(sqe) => {
                sqe.set_cancel_flags(flags);
                true
            }
            None => false,
        }
    }

    #[inline]
    pub unsafe fn prep_link_timeout(&mut self, ts: &libc::timespec, flags: u32) -> bool {
        match self
            .sq
            .prep_rw(Op::LinkTimeout, -1, ts as *const _ as *const _, 1, 0)
        {
            Some(sqe) => {
                sqe.set_timeout_flags(flags);
                true
            }
            None => false,
        }
    }

    #[inline]
    pub unsafe fn prep_connect(
        &mut self,
        fd: RawFd,
        addr: &libc::sockaddr,
        addr_len: libc::socklen_t,
    ) -> bool {
        self.sq
            .prep_rw(
                Op::Connect,
                fd,
                addr as *const _ as *const _,
                0,
                addr_len as u64,
            )
            .is_some()
    }

    #[inline]
    pub unsafe fn prep_files_update(&mut self, fds: &[RawFd], offset: u32) -> bool {
        self.sq
            .prep_rw(
                Op::FilesUpdate,
                -1,
                fds.as_ptr() as *const _,
                fds.len() as u32,
                offset as u64,
            )
            .is_some()
    }

    #[inline]
    pub unsafe fn prep_fallocate(&mut self, fd: RawFd, mode: u32, offset: u64, len: u64) -> bool {
        self.sq
            .prep_rw(Op::Fallocate, fd, len as *const _, mode, offset)
            .is_some()
    }

    #[inline]
    pub unsafe fn prep_openat(&mut self, dfd: RawFd, path: &CStr, flags: u32, mode: u32) -> bool {
        match self
            .sq
            .prep_rw(Op::Openat, dfd, path.as_ptr() as *const _, mode, 0)
        {
            Some(sqe) => {
                sqe.set_open_flags(flags);
                true
            }
            None => false,
        }
    }

    #[inline]
    pub unsafe fn prep_close(&mut self, fd: RawFd) -> bool {
        self.sq.prep_rw(Op::Close, fd, ptr::null(), 0, 0).is_some()
    }

    #[inline]
    pub unsafe fn prep_read(&mut self, fd: RawFd, buf: &mut [u8], offset: u64) -> bool {
        self.sq
            .prep_rw(
                Op::Read,
                fd,
                buf.as_ptr() as *const _,
                buf.len() as u32,
                offset,
            )
            .is_some()
    }

    #[inline]
    pub unsafe fn prep_write(&mut self, fd: RawFd, data: &[u8], offset: u64) -> bool {
        self.sq
            .prep_rw(
                Op::Write,
                fd,
                data.as_ptr() as *const _,
                data.len() as u32,
                offset,
            )
            .is_some()
    }

    #[inline]
    pub unsafe fn prep_statx(
        &mut self,
        dfd: RawFd,
        path: &CStr,
        flags: u32,
        mask: u32,
        statxbuf: &libc::statx,
    ) -> bool {
        match self.sq.prep_rw(
            Op::Statx,
            dfd,
            path.as_ptr() as *const _,
            mask,
            statxbuf as *const _ as u64,
        ) {
            Some(sqe) => {
                sqe.set_statx_flags(flags);
                true
            }
            None => false,
        }
    }

    #[inline]
    pub unsafe fn prep_fadvise(&mut self, fd: RawFd, offset: u64, len: u32, advice: i32) -> bool {
        match self.sq.prep_rw(Op::Fadvise, fd, ptr::null(), len, offset) {
            Some(sqe) => {
                sqe.set_fadvise_advice_flags(advice as u32);
                true
            }
            None => false,
        }
    }

    #[inline]
    pub unsafe fn prep_madvise(&mut self, mem: &[u8], advice: i32) -> bool {
        match self.sq.prep_rw(
            Op::Madvise,
            -1,
            mem.as_ptr() as *const _,
            mem.len() as u32,
            0,
        ) {
            Some(sqe) => {
                sqe.set_fadvise_advice_flags(advice as u32);
                true
            }
            None => false,
        }
    }

    #[inline]
    pub unsafe fn prep_send(&mut self, sockfd: RawFd, data: &[u8], flags: u32) -> bool {
        match self.sq.prep_rw(
            Op::Send,
            sockfd,
            data.as_ptr() as *const _,
            data.len() as u32,
            0,
        ) {
            Some(sqe) => {
                sqe.set_msg_flags(flags);
                true
            }
            None => false,
        }
    }

    #[inline]
    pub unsafe fn prep_recv(&mut self, sockfd: RawFd, buf: &mut [u8], flags: u32) -> bool {
        match self.sq.prep_rw(
            Op::Recv,
            sockfd,
            buf.as_ptr() as *const _,
            buf.len() as u32,
            0,
        ) {
            Some(sqe) => {
                sqe.set_msg_flags(flags);
                true
            }
            None => false,
        }
    }

    #[inline]
    pub unsafe fn prep_openat2(&mut self, dfd: RawFd, path: &CStr, how: &OpenHow) -> bool {
        self.sq
            .prep_rw(
                Op::Openat2,
                dfd,
                path.as_ptr() as *const _,
                mem::size_of::<OpenHow>() as u32,
                how as *const _ as u64,
            )
            .is_some()
    }

    #[inline]
    pub unsafe fn prep_epoll_ctl(
        &mut self,
        epfd: RawFd,
        fd: RawFd,
        op: u32,
        ev: &libc::epoll_event,
    ) -> bool {
        self.sq
            .prep_rw(
                Op::EpollCtl,
                epfd,
                ev as *const _ as *const _,
                op,
                fd as u64,
            )
            .is_some()
    }

    #[inline]
    pub unsafe fn prep_provide_buffers(
        &mut self,
        addr: *const libc::c_void,
        len: u32,
        nr: i32,
        bgid: u16,
        bid: u32,
    ) -> bool {
        match self
            .sq
            .prep_rw(Op::ProvideBuffers, nr, addr, len, bid as u64)
        {
            Some(sqe) => {
                sqe.set_buf_group(bgid);
                true
            }
            None => false,
        }
    }

    #[inline]
    pub unsafe fn prep_remove_buffer(&mut self, nr: i32, bgid: u16) -> bool {
        match self.sq.prep_rw(Op::RemoveBuffers, nr, ptr::null(), 0, 0) {
            Some(sqe) => {
                sqe.set_buf_group(bgid);
                true
            }
            None => false,
        }
    }

    #[inline]
    pub fn submit(&mut self) -> Result<usize> {
        self.submit_and_wait(0)
    }

    pub fn submit_and_wait(&mut self, wait_nr: u32) -> Result<usize> {
        let submitted = self.sq.flush();
        
        todo!()
    }
}
