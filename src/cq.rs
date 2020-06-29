use std::io::Result;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::params::UringParams;
use crate::sys;
use crate::uring::Mmap;

// struct io_cqring_offsets
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub(crate) struct Offsets {
    head: u32,
    tail: u32,
    ring_mask: u32,
    ring_entries: u32,
    overflow: u32,
    cqes: u32,
    flags: u32,
    _resv1: u32,
    _resv2: u64,
}

impl Offsets {
    #[inline]
    pub fn cqes(&self) -> u32 {
        self.cqes
    }
}

// IO completion data structure (Completion Queue Entry)
// struct io_uring_cqe
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Entry {
    user_data: u64, // sqe->data submission passed back
    res: i32,       // result code for this event
    flags: u32,     // IoRingCqeFlags::* flags
}

impl Entry {
    const F_BUFFER: u32 = 1 << 0;

    const BUFFER_SHIFT: u32 = 16;

    #[inline]
    pub fn user_data(&self) -> u64 {
        self.user_data
    }

    #[inline]
    pub fn res(&self) -> i32 {
        self.res
    }

    #[inline]
    pub fn buffer_id(&self) -> Option<u16> {
        if self.flags & Self::F_BUFFER != 0 {
            Some((self.flags >> Self::BUFFER_SHIFT) as u16)
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct Queue<'a> {
    khead: &'a AtomicU32,
    ktail: &'a AtomicU32,
    kring_mask: u32,
    kring_entries: u32,
    kflags: &'a AtomicU32,
    koverflow: &'a AtomicU32,
    cqes: *const Entry,

    khead_shadow: u32,
    ktail_shadow: u32,

    ring_ptr: Rc<Mmap<libc::c_void>>,
}

impl Queue<'_> {
    const UDATA_TIMEOUT: u64 = -1i64 as u64;

    #[inline]
    pub(crate) fn new(ring_ptr: Rc<Mmap<libc::c_void>>, params: &UringParams) -> Self {
        let ptr = ring_ptr.as_mut_ptr();
        let cq_off = params.cq_off();
        unsafe {
            let khead = &*(ptr.add(cq_off.head as usize) as *const AtomicU32);
            let ktail = &*(ptr.add(cq_off.tail as usize) as *const AtomicU32);
            Self {
                khead,
                ktail,
                kring_mask: *(ptr.add(cq_off.ring_mask as usize) as *const u32),
                kring_entries: *(ptr.add(cq_off.ring_entries as usize) as *const u32),
                kflags: &*(ptr.add(cq_off.flags as usize) as *const AtomicU32),
                koverflow: &*(ptr.add(cq_off.overflow as usize) as *const AtomicU32),
                cqes: ptr.add(cq_off.cqes as usize) as *const Entry,

                khead_shadow: khead.load(Ordering::Relaxed),
                ktail_shadow: ktail.load(Ordering::Acquire),

                ring_ptr,
            }
        }
    }

    #[inline]
    pub fn overflow(&self) -> u32 {
        self.koverflow.load(Ordering::Relaxed)
    }

    pub(crate) fn peek_cqe(&mut self) -> Result<Option<&Entry>> {
        loop {
            if self.khead_shadow == self.ktail_shadow {
                self.ktail_shadow = self.ktail.load(Ordering::Acquire);
                if self.khead_shadow == self.ktail_shadow {
                    return Ok(None);
                }
            }
            let cqe = unsafe {
                &*(self
                    .cqes
                    .add((self.khead_shadow & self.kring_mask) as usize))
            };
            if cqe.user_data == Self::UDATA_TIMEOUT {
                let err = cqe.res;
                self.advance(1);
                sys::cvt(err)?;
            } else {
                return Ok(Some(cqe));
            }
        }
    }

    #[inline]
    pub(crate) fn advance(&mut self, n: u32) {
        if n > 0 {
            self.khead_shadow = self.khead_shadow.wrapping_add(n);
            self.khead.store(self.khead_shadow, Ordering::Release);
        }
    }
}
