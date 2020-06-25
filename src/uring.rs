use std::alloc::{alloc_zeroed, Layout};
use std::ffi::CStr;
use std::fmt;
use std::io::{Error, IoSlice, IoSliceMut, Result};
use std::mem;
use std::os::unix::io::{AsRawFd, RawFd};
use std::ptr;

use bitflags::bitflags;

use crate::op::{Code, Op};
use crate::params::{Setup, UringBuilder};
use crate::{cq, sq, sys};

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

impl<T> Drop for Mmap<T> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            sys::munmap(self.addr.as_ptr() as *mut _, self.len).ok();
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub(crate) struct ProbeOp {
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

impl Probe {
    #[inline]
    pub fn support<T: Op>(&self) -> bool {
        const SUPPORTED: u16 = 1 << 0;
        if T::CODE <= self.last_op {
            let probe_op = unsafe { self.ops.get_unchecked(T::CODE as usize) };
            probe_op.flags & SUPPORTED != 0
        } else {
            false
        }
    }
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

// io_uring_enter(2) flags
bitflags! {
    pub struct Enter: u32 {
        const GETEVENTS = 1 << 0;
        const SQ_WAKEUP = 1 << 1;
    }
}

#[derive(Debug)]
pub struct Uring<'a> {
    sq: sq::Queue<'a>,
    cq: cq::Queue<'a>,
    flags: Setup,
    fd: Fd,
}

impl<'a> Uring<'a> {
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
    pub(crate) fn new(sq: sq::Queue<'a>, cq: cq::Queue<'a>, flags: Setup, fd: Fd) -> Self {
        Self { sq, cq, flags, fd }
    }

    #[inline]
    pub const fn entries(entries: u32) -> UringBuilder {
        UringBuilder::new(entries)
    }

    #[inline]
    pub(crate) unsafe fn register(&self, opcode: u32, arg: *const u8, nr_args: u32) -> Result<()> {
        sys::io_uring_register(self.fd.as_raw_fd(), opcode, arg, nr_args)
    }

    #[inline]
    pub unsafe fn register_buffers(&self, bufs: &[IoSliceMut]) -> Result<()> {
        self.register(
            Self::REGISTER_BUFFERS,
            bufs.as_ptr() as *const _,
            bufs.len() as u32,
        )
    }

    #[inline]
    pub unsafe fn unregister_buffers(&self) -> Result<()> {
        self.register(Self::UNREGISTER_BUFFERS, ptr::null(), 0)
    }

    #[inline]
    pub unsafe fn register_files(&self, fds: &[RawFd]) -> Result<()> {
        self.register(
            Self::REGISTER_FILES,
            fds.as_ptr() as *const _,
            fds.len() as u32,
        )
    }

    #[inline]
    pub unsafe fn unregister_files(&self) -> Result<()> {
        self.register(Self::UNREGISTER_FILES, ptr::null(), 0)
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

        self.register(
            Self::REGISTER_FILES_UPDATE,
            &fu as *const _ as *const _,
            fds.len() as u32,
        )
    }

    #[inline]
    pub unsafe fn register_eventfd(&self, event_fd: RawFd) -> Result<()> {
        self.register(Self::REGISTER_EVENTFD, &event_fd as *const _ as *const _, 1)
    }

    #[inline]
    pub unsafe fn unregister_eventfd(&self) -> Result<()> {
        self.register(Self::UNREGISTER_EVENTFD, ptr::null(), 0)
    }

    #[inline]
    pub unsafe fn register_eventfd_async(&self, event_fd: RawFd) -> Result<()> {
        self.register(
            Self::REGISTER_EVENTFD_ASYNC,
            &event_fd as *const _ as *const _,
            1,
        )
    }

    #[inline]
    pub unsafe fn register_personality(&self) -> Result<()> {
        self.register(Self::REGISTER_PERSONALITY, ptr::null(), 0)
    }

    #[inline]
    pub unsafe fn unregister_personality(&self, id: i32) -> Result<()> {
        self.register(Self::UNREGISTER_PERSONALITY, ptr::null(), id as u32)
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
            .prep_rw(Code::Splice as u8, fd_out, ptr::null(), nbytes, off_out)
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
                Code::Readv as u8,
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
            Code::ReadFixed as u8,
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
                Code::Writev as u8,
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
            Code::WriteFixed as u8,
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
            .prep_rw(Code::RecvMsg as u8, fd, msg as *const _ as *const _, 1, 0)
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
            .prep_rw(Code::SendMsg as u8, fd, msg as *const _ as *const _, 1, 0)
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
        match self.sq.prep_rw(Code::PollAdd as u8, fd, ptr::null(), 0, 0) {
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
            .prep_rw(Code::PollRemove as u8, fd, user_data, 0, 0)
            .is_some()
    }

    #[inline]
    pub unsafe fn prep_fsync(&mut self, fd: RawFd, fsync_flags: u32) -> bool {
        match self.sq.prep_rw(Code::Fsync as u8, fd, ptr::null(), 0, 0) {
            Some(sqe) => {
                sqe.set_fsync_flags(fsync_flags);
                true
            }
            None => false,
        }
    }

    #[inline]
    pub unsafe fn prep_timeout(&mut self, ts: &libc::timespec, count: u32, flags: u32) -> bool {
        match self.sq.prep_rw(
            Code::Timeout as u8,
            -1,
            ts as *const _ as *const _,
            1,
            count as u64,
        ) {
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
            .prep_rw(Code::TimeoutRemove as u8, -1, user_data as *const _, 0, 0)
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
            Code::Accept as u8,
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
        match self
            .sq
            .prep_rw(Code::AsyncCancel as u8, -1, user_data, 0, 0)
        {
            Some(sqe) => {
                sqe.set_cancel_flags(flags);
                true
            }
            None => false,
        }
    }

    #[inline]
    pub unsafe fn prep_link_timeout(&mut self, ts: &libc::timespec, flags: u32) -> bool {
        match self.sq.prep_rw(
            Code::LinkTimeout as u8,
            -1,
            ts as *const _ as *const _,
            1,
            0,
        ) {
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
                Code::Connect as u8,
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
                Code::FilesUpdate as u8,
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
            .prep_rw(Code::Fallocate as u8, fd, len as *const _, mode, offset)
            .is_some()
    }

    #[inline]
    pub unsafe fn prep_openat(&mut self, dfd: RawFd, path: &CStr, flags: u32, mode: u32) -> bool {
        match self
            .sq
            .prep_rw(Code::Openat as u8, dfd, path.as_ptr() as *const _, mode, 0)
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
        self.sq
            .prep_rw(Code::Close as u8, fd, ptr::null(), 0, 0)
            .is_some()
    }

    #[inline]
    pub unsafe fn prep_read(&mut self, fd: RawFd, buf: &mut [u8], offset: u64) -> bool {
        self.sq
            .prep_rw(
                Code::Read as u8,
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
                Code::Write as u8,
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
            Code::Statx as u8,
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
        match self
            .sq
            .prep_rw(Code::Fadvise as u8, fd, ptr::null(), len, offset)
        {
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
            Code::Madvise as u8,
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
            Code::Send as u8,
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
            Code::Recv as u8,
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
                Code::Openat2 as u8,
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
                Code::EpollCtl as u8,
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
            .prep_rw(Code::ProvideBuffers as u8, nr, addr, len, bid as u64)
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
        match self
            .sq
            .prep_rw(Code::RemoveBuffers as u8, nr, ptr::null(), 0, 0)
        {
            Some(sqe) => {
                sqe.set_buf_group(bgid);
                true
            }
            None => false,
        }
    }

    #[inline]
    pub fn submit(&mut self) -> Result<u32> {
        self.submit_and_wait(0)
    }

    pub fn submit_and_wait(&mut self, wait_nr: u32) -> Result<u32> {
        let submitted = self.sq.flush();
        let mut flags = Enter::empty();
        let n = if self.need_enter(submitted, &mut flags) || wait_nr > 0 {
            if wait_nr > 0 || self.flags.contains(Setup::IOPOLL) {
                flags.insert(Enter::GETEVENTS);
            }
            self.enter(submitted, wait_nr, &flags)?
        } else {
            submitted
        };
        Ok(n)
    }

    #[inline]
    pub fn wait_cqe_nr(&mut self, wait_nr: u32) -> Result<cq::Entry> {
        self.get_cqe(0, wait_nr, None)
    }

    #[inline]
    pub fn wait_cqe(&mut self) -> Result<cq::Entry> {
        self.get_cqe(0, 1, None)
    }

    #[inline]
    pub fn sq_dropped(&self) -> u32 {
        self.sq.dropped()
    }

    #[inline]
    pub fn cq_overflow(&self) -> u32 {
        self.cq.overflow()
    }

    #[inline]
    pub(crate) fn sq(&mut self) -> &mut sq::Queue<'a> {
        &mut self.sq
    }

    fn get_cqe(
        &mut self,
        mut submit: u32,
        to_wait: u32,
        sigmask: Option<&libc::sigset_t>,
    ) -> Result<cq::Entry> {
        let mut wait_nr = to_wait;
        let mut ret = 0;
        loop {
            let mut flags = Enter::empty();
            let peeked = match self.cq.peek_cqe()? {
                Some(cqe) => {
                    if wait_nr > 0 {
                        wait_nr -= 1;
                    }
                    Some(*cqe)
                }
                None => {
                    if to_wait == 0 && submit == 0 {
                        return Err(Error::from_raw_os_error(libc::EAGAIN));
                    }
                    None
                }
            };

            if wait_nr > 0 {
                flags.insert(Enter::GETEVENTS);
            }
            if submit > 0 {
                self.need_enter(submit, &mut flags);
            }
            if wait_nr > 0 || submit > 0 {
                ret = self.penter(submit, wait_nr, &flags, sigmask)?
            }
            if ret == submit {
                submit = 0;
                if !self.flags.contains(Setup::IOPOLL) {
                    wait_nr = 0;
                }
            } else {
                submit -= ret;
            }
            if let Some(cqe) = peeked {
                self.cq.advance(1);
                return Ok(cqe);
            }
        }
    }

    #[inline]
    fn need_enter(&self, submitted: u32, flags: &mut Enter) -> bool {
        if !self.flags.contains(Setup::SQPOLL) && submitted > 0 {
            return true;
        }
        if self.sq.need_wakeup() {
            flags.insert(Enter::SQ_WAKEUP);
            return true;
        }
        false
    }

    #[inline]
    fn enter(&self, to_submit: u32, min_complete: u32, flags: &Enter) -> Result<u32> {
        unsafe { sys::io_uring_enter(self.fd.as_raw_fd(), to_submit, min_complete, flags.bits()) }
    }

    #[inline]
    fn penter(
        &self,
        to_submit: u32,
        min_complete: u32,
        flags: &Enter,
        sig: Option<&libc::sigset_t>,
    ) -> Result<u32> {
        match sig {
            Some(s) => unsafe {
                sys::io_uring_penter(
                    self.fd.as_raw_fd(),
                    to_submit,
                    min_complete,
                    flags.bits(),
                    s,
                )
            },
            None => unsafe {
                sys::io_uring_enter(self.fd.as_raw_fd(), to_submit, min_complete, flags.bits())
            },
        }
    }
}
