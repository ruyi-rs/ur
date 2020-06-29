use std::alloc::{alloc_zeroed, Layout};
use std::fmt;
use std::io::{Error, IoSliceMut, Result};
use std::os::unix::io::{AsRawFd, RawFd};
use std::ptr;
use std::time::Duration;

use bitflags::bitflags;

use crate::op::{self, Op};
use crate::params::{Setup, UringBuilder};
use crate::{cq, sq, sys};

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
    ts: libc::timespec,
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
        Self {
            sq,
            cq,
            flags,
            fd,
            ts: libc::timespec {
                tv_sec: 0,
                tv_nsec: 0,
            },
        }
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

    pub fn wait_cqes(
        &mut self,
        wait_nr: u32,
        timeout: Option<Duration>,
        sigmask: Option<&libc::sigset_t>,
    ) -> Result<cq::Entry> {
        let mut to_submit = 0;
        if let Some(dur) = timeout {
            self.ts = libc::timespec {
                tv_sec: dur.as_secs() as libc::time_t,
                tv_nsec: dur.subsec_nanos() as libc::c_long,
            };
            match unsafe {
                op::Timeout {
                    ts: &self.ts,
                    count: wait_nr,
                    flags: 0,
                }
                .prepare(&mut self.sq)
            } {
                Some(sqe) => {
                    sqe.set_user_data(cq::Queue::UDATA_TIMEOUT);
                    to_submit = self.sq.flush();
                }
                None => return Err(Error::from_raw_os_error(libc::EAGAIN)),
            }
        }

        self.get_cqe(to_submit, wait_nr, sigmask)
    }

    #[inline]
    pub fn wait_cqe_timeout(&mut self, timeout: Option<Duration>) -> Result<cq::Entry> {
        self.wait_cqes(1, timeout, None)
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
    pub fn sq_mut(&mut self) -> &mut sq::Queue<'a> {
        &mut self.sq
    }

    #[inline]
    pub fn sq(&self) -> &sq::Queue<'a> {
        &self.sq
    }

    #[inline]
    pub fn cq(&self) -> &cq::Queue<'a> {
        &self.cq
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

                // When Setup::IOPOLL is set, sys::io_uring enter()
                // must be called to reap new completions but the call
                // won't be made if both wait_nr and submit are zero
                // so preserve wait_nr.
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
