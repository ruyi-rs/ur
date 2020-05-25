use std::mem::MaybeUninit;
use std::os::unix::io::RawFd;

use bitflags::bitflags;

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

// IoUringParams->features flags
bitflags! {
    pub struct IoRingFeat: u32 {
        const SINGLE_MMAP       = 1 << 0;
        const NODROP            = 1 << 1;
        const SUBMIT_STABLE     = 1 << 2;
        const RW_CUR_POS        = 1 << 3;
        const CUR_PERSONALITY   = 1 << 4;
        const FAST_POLL         = 1 << 5;
    }
}

// Filled with the offset for mmap(2)
// struct io_sqring_offsets
#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct IoSqringOffsets {
    head: u32,
    tail: u32,
    ring_mask: u32,
    ring_entries: u32,
    flags: u32, // IoRingSq::* flags
    dropped: u32,
    array: u32,
    _resv1: u32,
    _resv2: u64,
}

// struct io_cqring_offsets
#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct IoCqringOffsets {
    head: u32,
    tail: u32,
    ring_mask: u32,
    ring_entries: u32,
    overflow: u32,
    cqes: u32,
    _resv: u64,
}

// Passed in for io_uring_setup(2). Copied back with updated info on success
// struct io_uring_params
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct IoUringParams {
    sq_entries: u32,
    cq_entries: u32,
    flags: u32, // IORING_SETUP_ flags (IoRingSetup::*)
    sq_thread_cpu: u32,
    sq_thread_idle: u32,
    features: u32, // IoRingFeat::* flags
    wq_fd: u32,
    _resv: [u32; 3],
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

    #[inline]
    pub fn features(&self) -> IoRingFeat {
        unsafe { IoRingFeat::from_bits_unchecked(self.features) }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Builder {
    cq_entries: u32,
    flags: IoRingSetup,
    sq_thread_cpu: u32,
    sq_thread_idle: u32,
    wq_fd: RawFd,
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
        self.sq_thread_idle = idle;
        self
    }

    #[inline]
    pub fn sqpoll_cpu(&mut self, cpu: u32) -> &mut Self {
        self.sqpoll();
        self.flags |= IoRingSetup::SQ_AFF;
        self.sq_thread_cpu = cpu;
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
        self.wq_fd = wq_fd;
        self
    }

    #[inline]
    pub fn build(&self) -> IoUringParams {
        let mut params: IoUringParams = unsafe { MaybeUninit::zeroed().assume_init() };
        params.cq_entries = self.cq_entries;
        params.flags = self.flags.bits();
        params.sq_thread_cpu = self.sq_thread_cpu;
        params.sq_thread_idle = self.sq_thread_idle;
        params.wq_fd = self.wq_fd as u32;
        params
    }
}
