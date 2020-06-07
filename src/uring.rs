use std::alloc::{alloc_zeroed, Layout};
use std::fmt;
use std::io::{IoSlice, Result};
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
    pub const fn as_ptr(&self) -> *mut T {
        self.addr.as_ptr()
    }
}

impl<T> Drop for Mmap<T> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            let ptr = self.addr.as_ptr() as *mut libc::c_void;
            sys::munmap(ptr, self.len).ok();
        }
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
    pub fn register_buffers(&self, bufs: &[IoSlice]) -> Result<()> {
        unsafe {
            sys::io_uring_register(
                self.fd.as_raw_fd(),
                Self::REGISTER_BUFFERS,
                bufs.as_ptr() as *const _,
                bufs.len() as u32,
            )
        }
    }

    #[inline]
    pub fn unregister_buffers(&self) -> Result<()> {
        unsafe {
            sys::io_uring_register(
                self.fd.as_raw_fd(),
                Self::UNREGISTER_BUFFERS,
                ptr::null(),
                0,
            )
        }
    }

    #[inline]
    pub fn register_files(&self, fds: &[RawFd]) -> Result<()> {
        unsafe {
            sys::io_uring_register(
                self.fd.as_raw_fd(),
                Self::REGISTER_FILES,
                fds.as_ptr() as *const _,
                fds.len() as u32,
            )
        }
    }

    #[inline]
    pub fn unregister_files(&self) -> Result<()> {
        unsafe {
            sys::io_uring_register(self.fd.as_raw_fd(), Self::UNREGISTER_FILES, ptr::null(), 0)
        }
    }

    #[inline]
    pub fn register_files_update(&self, offset: u32, fds: &[RawFd]) -> Result<()> {
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

        unsafe {
            sys::io_uring_register(
                self.fd.as_raw_fd(),
                Self::REGISTER_FILES_UPDATE,
                &fu as *const _ as *const _,
                fds.len() as u32,
            )
        }
    }

    #[inline]
    pub fn register_eventfd(&self, event_fd: RawFd) -> Result<()> {
        unsafe {
            sys::io_uring_register(
                self.fd.as_raw_fd(),
                Self::REGISTER_EVENTFD,
                &event_fd as *const _ as *const _,
                1,
            )
        }
    }

    #[inline]
    pub fn unregister_eventfd(&self) -> Result<()> {
        unsafe {
            sys::io_uring_register(
                self.fd.as_raw_fd(),
                Self::UNREGISTER_EVENTFD,
                ptr::null(),
                0,
            )
        }
    }

    #[inline]
    pub fn register_eventfd_async(&self, event_fd: RawFd) -> Result<()> {
        unsafe {
            sys::io_uring_register(
                self.fd.as_raw_fd(),
                Self::REGISTER_EVENTFD_ASYNC,
                &event_fd as *const _ as *const _,
                1,
            )
        }
    }

    #[inline]
    pub fn register_personality(&self) -> Result<()> {
        unsafe {
            sys::io_uring_register(
                self.fd.as_raw_fd(),
                Self::REGISTER_PERSONALITY,
                ptr::null(),
                0,
            )
        }
    }

    #[inline]
    pub fn unregister_personality(&self, id: i32) -> Result<()> {
        unsafe {
            sys::io_uring_register(
                self.fd.as_raw_fd(),
                Self::UNREGISTER_PERSONALITY,
                ptr::null(),
                id as u32,
            )
        }
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
    pub fn prep_splice(
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
    pub fn prep_readv(&mut self, fd: RawFd, iovecs: &[IoSlice], offset: u64) -> bool {
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
}
