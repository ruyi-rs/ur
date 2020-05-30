use std::cmp;
use std::io::Result;
use std::mem::{self, MaybeUninit};

use bitflags::bitflags;

use crate::uring::{Fd, Uring, Pointer};
use crate::{cq, sq, sys};

// io_uring_setup() flags
// IORING_SETUP_ flags
bitflags! {
    pub struct Setup: u32 {
        const IOPOLL    = 1 << 0; // io_context is polled
        const SQPOLL    = 1 << 1; // SQ poll thread
        const SQ_AFF    = 1 << 2; // sq_thread_cpu is valid
        const CQSIZE    = 1 << 3; // app defines CQ size
        const CLAMP     = 1 << 4; // clamp SQ/CQ ring sizes
        const ATTACH_WQ = 1 << 5; // attach to existing wq
    }
}

// UringParams->features flags
bitflags! {
    pub struct Feat: u32 {
        const SINGLE_MMAP       = 1 << 0;
        const NODROP            = 1 << 1;
        const SUBMIT_STABLE     = 1 << 2;
        const RW_CUR_POS        = 1 << 3;
        const CUR_PERSONALITY   = 1 << 4;
        const FAST_POLL         = 1 << 5;
    }
}

// Passed in for io_uring_setup(2). Copied back with updated info on success
// struct io_uring_params
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub(crate) struct UringParams {
    sq_entries: u32,
    cq_entries: u32,
    flags: u32, // IORING_SETUP_ flags (Setup::*)
    sq_thread_cpu: u32,
    sq_thread_idle: u32,
    features: u32, // Feat::* flags
    wq_fd: u32,
    _resv: [u32; 3],
    sq_off: sq::Offsets,
    cq_off: cq::Offsets,
}

impl UringParams {
    // Magic offsets for the application to mmap the data it needs
    const IORING_OFF_SQ_RING: i64 = 0;
    const IORING_OFF_CQ_RING: i64 = 0x800_0000;
    const IORING_OFF_SQES: i64 = 0x1000_0000;

    #[inline]
    pub fn flags(&self) -> Setup {
        unsafe { Setup::from_bits_unchecked(self.flags) }
    }

    #[inline]
    pub fn features(&self) -> Feat {
        unsafe { Feat::from_bits_unchecked(self.features) }
    }

    #[inline]
    pub const fn sq_off(&self) -> &sq::Offsets {
        &self.sq_off
    }

    #[inline]
    pub fn cq_off(&self) -> &cq::Offsets {
        &self.cq_off
    }

    #[inline]
    fn mmap(&self, fd: &Fd) -> Result<(sq::Queue, cq::Queue)> {
        let sq_ring_sz =
            self.sq_off.array() as usize + self.sq_entries as usize * mem::size_of::<u32>();
        let cq_ring_sz =
            self.cq_off.cqes() as usize + self.cq_entries as usize * mem::size_of::<cq::Entry>();

        let (sq_ring_ptr, cq_ring_ptr) = if self.features().contains(Feat::SINGLE_MMAP) {
            let ring_sz = cmp::max(sq_ring_sz, cq_ring_sz);
            let sq_ring_ptr =
                Pointer::<libc::c_void>::try_new(ring_sz, &fd, Self::IORING_OFF_SQ_RING)?;
            let cq_ring_ptr = cq::RingPtr::from_ref(&sq_ring_ptr);
            (sq_ring_ptr, cq_ring_ptr)
        } else {
            let sq_ring_ptr =
                Pointer::<libc::c_void>::try_new(sq_ring_sz, &fd, Self::IORING_OFF_SQ_RING)?;
            let cq_ring_ptr =
                Pointer::<libc::c_void>::try_new(cq_ring_sz, &fd, Self::IORING_OFF_CQ_RING)?;
            (sq_ring_ptr, cq::RingPtr::from_owned(cq_ring_ptr))
        };
        let sqes_sz = self.sq_entries as usize * mem::size_of::<sq::Entry>();
        let sqes = Pointer::<sq::Entry>::try_new(sqes_sz, &fd, Self::IORING_OFF_SQES)?;
        let sq = sq::Queue::new(sq_ring_ptr, sqes, self);
        let cq = cq::Queue::new(cq_ring_ptr, self);
        Ok((sq, cq))
    }
}

#[derive(Debug, Copy, Clone)]
pub struct UringBuilder {
    entries: u32,
    cq_entries: u32,
    flags: Setup,
    sq_thread_cpu: u32,
    sq_thread_idle: u32,
    wq_fd: u32,
}

impl UringBuilder {
    #[inline]
    pub(crate) const fn new(entries: u32) -> Self {
        Self {
            entries,
            cq_entries: 0,
            flags: Setup::empty(),
            sq_thread_cpu: 0,
            sq_thread_idle: 0,
            wq_fd: 0,
        }
    }

    #[inline]
    pub fn iopoll(&mut self) -> &mut Self {
        self.flags |= Setup::IOPOLL;
        self
    }

    #[inline]
    pub fn sqpoll(&mut self) -> &mut Self {
        self.flags |= Setup::SQPOLL;
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
        self.flags |= Setup::SQ_AFF;
        self.sq_thread_cpu = cpu;
        self
    }

    #[inline]
    pub fn cqsize(&mut self, cq_entries: u32) -> &mut Self {
        self.flags |= Setup::CQSIZE;
        self.cq_entries = cq_entries;
        self
    }

    #[inline]
    pub fn clamp(&mut self) -> &mut Self {
        self.flags |= Setup::CLAMP;
        self
    }

    #[inline]
    pub fn attach_wq(&mut self, wq_fd: u32) -> &mut Self {
        self.flags |= Setup::ATTACH_WQ;
        self.wq_fd = wq_fd;
        self
    }

    pub fn try_build(&self) -> Result<Uring> {
        let mut params = self.params();
        let fd = self.setup(&mut params)?;
        let (sq, cq) = params.mmap(&fd)?;
        let uring = Uring::new(sq, cq, params.flags(), fd);
        Ok(uring)
    }

    #[inline]
    fn params(&self) -> UringParams {
        let mut params: UringParams = unsafe { MaybeUninit::zeroed().assume_init() };
        params.cq_entries = self.cq_entries;
        params.flags = self.flags.bits();
        params.sq_thread_cpu = self.sq_thread_cpu;
        params.sq_thread_idle = self.sq_thread_idle;
        params.wq_fd = self.wq_fd;
        params
    }

    #[inline]
    fn setup(&self, params: &mut UringParams) -> Result<Fd> {
        let fd = unsafe { sys::io_uring_setup(self.entries, params)? };
        Ok(Fd::new(fd))
    }
}
