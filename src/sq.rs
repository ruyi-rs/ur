use std::fmt;
use std::io::Result;
use std::ptr;

use libc;

use crate::params::IoUringParams;

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
struct Entry {
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
    sqes: *const Entry,

    sqe_head: u32,
    sqe_tail: u32,

    ring_sz: usize,
    ring_ptr: *const libc::c_void, 
}

impl Queue {

    pub fn try_new(ring_fd: i32, params: &IoUringParams) -> Result<Self> {
        
        let q = Self {
            khead: ptr::null(),
            ktail: ptr::null_mut(),
            kring_mask: ptr::null(),
            kring_entries: ptr::null(),
            kflags: ptr::null(),
            kdropped: ptr::null(),
            array: ptr::null(),
            sqes: ptr::null(),
            sqe_head: 0,
            sqe_tail: 0,
            ring_sz: 0,
            ring_ptr: ptr::null(),
        };
        Ok(q)
    }
}
