use std::ptr::NonNull;

use crate::params::IoUringParams;
use crate::uring::Pointer;

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

#[derive(Debug)]
pub(crate) enum RingPtr {
    Ptr(NonNull<libc::c_void>),
    Owned(Pointer<libc::c_void>),
}

impl RingPtr {
    #[inline]
    pub const fn from_ref(ptr: &Pointer<libc::c_void>) -> Self {
        RingPtr::Ptr(unsafe { NonNull::new_unchecked(ptr.as_ptr()) })
    }

    #[inline]
    pub const fn from_owned(ptr: Pointer<libc::c_void>) -> Self {
        RingPtr::Owned(ptr)
    }

    #[inline]
    fn as_ptr(&self) -> *mut libc::c_void {
        match *self {
            Self::Ptr(p) => p.as_ptr(),
            Self::Owned(ref p) => p.as_ptr(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct Queue {
    khead: *mut u32,
    ktail: *const u32,
    kring_mask: *const u32,
    kring_entries: *const u32,
    kflags: *const u32,
    koverflow: *const u32,
    cqes: *const Entry,

    ring_ptr: RingPtr,
}

impl Queue {
    #[inline]
    pub fn new(ring_ptr: RingPtr, params: &IoUringParams) -> Self {
        let ptr = ring_ptr.as_ptr();
        let cq_off = params.cq_off();
        unsafe {
            Self {
                khead: ptr.add(cq_off.head as usize) as *mut u32,
                ktail: ptr.add(cq_off.tail as usize) as *const u32,
                kring_mask: ptr.add(cq_off.ring_mask as usize) as *const u32,
                kring_entries: ptr.add(cq_off.ring_entries as usize) as *const u32,
                kflags: ptr.add(cq_off.flags as usize) as *const u32,
                koverflow: ptr.add(cq_off.overflow as usize) as *const u32,
                cqes: ptr.add(cq_off.cqes as usize) as *const Entry,
                ring_ptr,
            }
        }
    }
}
