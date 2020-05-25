use std::mem::MaybeUninit;
use std::os::unix::io::RawFd;

use bitflags::bitflags;
use libc;

// io_uring_setup() flags
// IORING_SETUP_ flags
bitflags! {
    pub struct IoRingSetup: u32 {
        const IOPOLL    = 1 << 0; // io_context is polled
        const SQPOLL    = 1 << 1; // SQ poll thread
        const SQ_AFF    = 1 << 2; // sq_thread_cpu is valid
        const CQSIZE    = 1 << 3; // app defines CQ size
        const CLAMP     = 1 << 4; // clamp SQ/CQ ring sizes
        const ATTACH_WQ = 1 << 5; // attach to existing wq
    }
}

// Filled with the offset for mmap(2)
// struct io_sqring_offsets
#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct IoSqringOffsets {
    head: libc::__u32,
    tail: libc::__u32,
    ring_mask: libc::__u32,
    ring_entries: libc::__u32,
    flags: libc::__u32, // IoRingSq::* flags
    dropped: libc::__u32,
    array: libc::__u32,
    _resv1: libc::__u32,
    _resv2: libc::__u64,
}

// struct io_cqring_offsets
#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct IoCqringOffsets {
    head: libc::__u32,
    tail: libc::__u32,
    ring_mask: libc::__u32,
    ring_entries: libc::__u32,
    overflow: libc::__u32,
    cqes: libc::__u32,
    _resv: libc::__u64,
}

// Passed in for io_uring_setup(2). Copied back with updated info on success
// struct io_uring_params
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct IoUringParams {
    sq_entries: libc::__u32,
    cq_entries: libc::__u32,
    flags: libc::__u32, // IORING_SETUP_ flags (IoRingSetup::*)
    sq_thread_cpu: libc::__u32,
    sq_thread_idle: libc::__u32,
    features: libc::__u32, // IoRingFeat::* flags
    wq_fd: libc::__u32,
    _resv: [libc::__u32; 3],
    sq_off: IoSqringOffsets,
    cq_off: IoCqringOffsets,
}

impl IoUringParams {
    #[inline]
    pub const fn builder() -> Builder {
        Builder::new()
    }

    #[inline]
    pub fn flags(&self) -> IoRingSetup {
        unsafe { IoRingSetup::from_bits_unchecked(self.flags) }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Builder {
    cq_entries: libc::__u32,
    flags: IoRingSetup,
    sq_thread_cpu: libc::__u32,
    sq_thread_idle: libc::__u32,
    wq_fd: libc::__u32,
}

impl Builder {
    #[inline]
    const fn new() -> Self {
        Self {
            cq_entries: 0,
            flags: IoRingSetup::empty(),
            sq_thread_cpu: 0,
            sq_thread_idle: 0,
            wq_fd: 0,
        }
    }

    #[inline]
    pub fn iopoll(&mut self) -> &mut Self {
        self.flags |= IoRingSetup::IOPOLL;
        self
    }

    #[inline]
    pub fn sqpoll(&mut self) -> &mut Self {
        self.flags |= IoRingSetup::SQPOLL;
        self
    }

    #[inline]
    pub fn sqpoll_idle(&mut self, idle: u32) -> &mut Self {
        self.sqpoll();
        self.sq_thread_idle = idle as libc::__u32;
        self
    }

    #[inline]
    pub fn sqpoll_cpu(&mut self, cpu: u32) -> &mut Self {
        self.sqpoll();
        self.flags |= IoRingSetup::SQ_AFF;
        self.sq_thread_cpu = cpu as libc::__u32;
        self
    }

    #[inline]
    pub fn cqsize(&mut self, cq_entries: u32) -> &mut Self {
        self.flags |= IoRingSetup::CQSIZE;
        self.cq_entries = cq_entries;
        self
    }

    #[inline]
    pub fn clamp(&mut self) -> &mut Self {
        self.flags |= IoRingSetup::CLAMP;
        self
    }

    #[inline]
    pub fn attach_wq(&mut self, wq_fd: RawFd) -> &mut Self {
        self.flags |= IoRingSetup::ATTACH_WQ;
        self.wq_fd = wq_fd as libc::__u32;
        self
    }

    #[inline]
    pub fn build(&self) -> IoUringParams {
        let mut params: IoUringParams = unsafe { MaybeUninit::zeroed().assume_init() };
        params.cq_entries = self.cq_entries;
        params.flags = self.flags.bits();
        params.sq_thread_cpu = self.sq_thread_cpu;
        params.sq_thread_idle = self.sq_thread_idle;
        params.wq_fd = self.wq_fd;
        params
    }
}
