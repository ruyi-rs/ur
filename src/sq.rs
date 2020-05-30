use std::fmt;

use crate::params::UringParams;
use crate::uring::Mmap;

// Filled with the offset for mmap(2)
// struct io_sqring_offsets
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Offsets {
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

impl Offsets {
    #[inline]
    pub fn array(&self) -> u32 {
        self.array
    }
}

#[allow(non_camel_case_types)]
type __kernel_rwf_t = i32; // libc::c_int

#[repr(C)]
#[derive(Copy, Clone)]
union OpFlags {
    rw: __kernel_rwf_t,
    fsync: libc::__u32, // IoRingFsync::* flags
    poll_events: libc::__u16,
    sync_range: libc::__u32,
    msg: libc::__u32,
    timeout: libc::__u32, // IoRingTimeout::* flags
    accept: libc::__u32,
    cancel: libc::__u32,
    open: libc::__u32,
    statx: libc::__u32,
    fadvise_advice: libc::__u32,
    splice: libc::__u32, // SpliceFlags::*
}

impl fmt::Debug for OpFlags {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#x}", unsafe { self.rw })
    }
}

// IO submission data structure (Submission Queue Entry)
// struct io_uring_sqe
#[repr(C)]
#[derive(Debug)]
pub struct Entry {
    opcode: u8,              // type of operation for this sqe
    flags: u8,               // IOSQE_ flags (IoSqe::*)
    ioprio: u16,             // ioprio for the request
    fd: i32,                 // file descriptor to do IO on
    off_addr2: u64,          // offset into file
    addr_splice_off_in: u64, // pointer to buffer or iovecs
    len: u32,                // buffer size or number of iovecs
    op_flags: OpFlags,
    user_data: u64, // data to be passed back at completion time

    // [u64; 3]
    buf_index_group: u16, // index into fixed buffers, if used; for grouped buffer selection
    personality: u16,     // personality to use, if used
    splice_fd_in: i32,
    _pad2: [u64; 2],
}

#[derive(Debug)]
pub struct Queue {
    khead: *const u32,
    ktail: *mut u32,
    kring_mask: *const u32,
    kring_entries: *const u32,
    kflags: *const u32,
    kdropped: *const u32,
    array: *const u32,
    sqes: Mmap<Entry>,

    sqe_head: u32,
    sqe_tail: u32,

    ring_ptr: Mmap<libc::c_void>,
}

impl Queue {
    #[inline]
    pub(crate) fn new(
        ring_ptr: Mmap<libc::c_void>,
        sqes: Mmap<Entry>,
        params: &UringParams,
    ) -> Self {
        let ptr = ring_ptr.as_ptr();
        let sq_off = params.sq_off();
        unsafe {
            Self {
                khead: ptr.add(sq_off.head as usize) as *const u32,
                ktail: ptr.add(sq_off.tail as usize) as *mut u32,
                kring_mask: ptr.add(sq_off.ring_mask as usize) as *const u32,
                kring_entries: ptr.add(sq_off.ring_entries as usize) as *const u32,
                kflags: ptr.add(sq_off.flags as usize) as *const u32,
                kdropped: ptr.add(sq_off.dropped as usize) as *const u32,
                array: ptr.add(sq_off.array as usize) as *const u32,
                sqes,
                sqe_head: 0,
                sqe_tail: 0,
                ring_ptr,
            }
        }
    }
}
