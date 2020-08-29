use std::fmt;
use std::os::unix::io::RawFd;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::params::UringParams;
use crate::uring::Mmap;

// Filled with the offset for mmap(2)
// struct io_sqring_offsets
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub(crate) struct Offsets {
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
    poll_events: libc::__u32,
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

impl Entry {
    #[inline]
    pub(crate) fn set_splice_off_in(&mut self, splice_off_in: u64) {
        self.addr_splice_off_in = splice_off_in;
    }

    #[inline]
    pub(crate) fn set_splice_fd_in(&mut self, splice_fd_in: RawFd) {
        self.splice_fd_in = splice_fd_in;
    }

    #[inline]
    pub(crate) fn set_buf_index(&mut self, buf_index: u16) {
        self.buf_index_group = buf_index;
    }

    #[inline]
    pub(crate) fn set_buf_group(&mut self, buf_group: u16) {
        self.buf_index_group = buf_group;
    }

    #[inline]
    pub(crate) fn set_fsync_flags(&mut self, fsync_flags: u32) {
        self.op_flags.fsync = fsync_flags;
    }

    #[inline]
    pub(crate) fn set_poll_events(&mut self, poll_events: u32) {
        self.op_flags.poll_events = if cfg!(target_endian = "big") {
            poll_events.reverse_bits()
        } else {
            poll_events
        }
    }

    #[inline]
    pub(crate) fn set_sync_range_flags(&mut self, sync_rang_flags: u32) {
        self.op_flags.sync_range = sync_rang_flags;
    }

    #[inline]
    pub(crate) fn set_msg_flags(&mut self, msg_flags: u32) {
        self.op_flags.msg = msg_flags;
    }

    #[inline]
    pub(crate) fn set_timeout_flags(&mut self, timeout_flags: u32) {
        self.op_flags.timeout = timeout_flags;
    }

    #[inline]
    pub(crate) fn set_accept_flags(&mut self, accept_flags: u32) {
        self.op_flags.accept = accept_flags;
    }

    #[inline]
    pub(crate) fn set_cancel_flags(&mut self, cancel_flags: u32) {
        self.op_flags.cancel = cancel_flags;
    }

    #[inline]
    pub(crate) fn set_open_flags(&mut self, open_flags: u32) {
        self.op_flags.open = open_flags;
    }

    #[inline]
    pub(crate) fn set_statx_flags(&mut self, statx_flags: u32) {
        self.op_flags.statx = statx_flags;
    }

    #[inline]
    pub(crate) fn set_fadvise_advice_flags(&mut self, fadvise_advice_flags: u32) {
        self.op_flags.fadvise_advice = fadvise_advice_flags;
    }

    #[inline]
    pub(crate) fn set_splice_flags(&mut self, splice_flags: u32) {
        self.op_flags.splice = splice_flags;
    }

    #[inline]
    pub fn set_user_data(&mut self, user_data: u64) {
        self.user_data = user_data;
    }
}

#[derive(Debug)]
pub struct Queue<'a> {
    khead: &'a AtomicU32,
    ktail: &'a AtomicU32,
    kring_mask: u32,
    kring_entries: u32,
    kflags: &'a AtomicU32,
    kdropped: &'a AtomicU32,
    //array: *const u32,
    sqes: Mmap<Entry>,

    khead_shadow: u32,
    ktail_shadow: u32,

    sqe_head: u32,
    sqe_tail: u32,

    ring_ptr: Rc<Mmap<libc::c_void>>,
}

impl Queue<'_> {
    // needs io_uring_enter wakeup
    const NEED_WAKEUP: u32 = 1 << 0;
    // CQ ring is overflow
    const CQ_OVERFLOW: u32 = 1 << 1;

    #[inline]
    pub(crate) fn new(
        ring_ptr: Rc<Mmap<libc::c_void>>,
        sqes: Mmap<Entry>,
        params: &UringParams,
    ) -> Self {
        let ptr = ring_ptr.as_mut_ptr();
        let sq_off = params.sq_off();
        unsafe {
            let khead = &*(ptr.add(sq_off.head as usize) as *const AtomicU32);
            let ktail = &*(ptr.add(sq_off.tail as usize) as *const AtomicU32);
            let array = ptr.add(sq_off.array as usize) as *mut u32;
            let kring_mask = *(ptr.add(sq_off.ring_mask as usize) as *const u32);
            let kring_entries = *(ptr.add(sq_off.ring_entries as usize) as *const u32);
            let ktail_shadow = ktail.load(Ordering::Relaxed);

            let mut i = ktail_shadow;
            for head in 0..kring_entries {
                *(array.add((i & kring_mask) as usize)) = head;
                i = i.wrapping_add(1);
            }

            Self {
                khead,
                ktail,
                kring_mask,
                kring_entries,
                kflags: &*(ptr.add(sq_off.flags as usize) as *const AtomicU32),
                kdropped: &*(ptr.add(sq_off.dropped as usize) as *const AtomicU32),
                //array,
                sqes,
                khead_shadow: khead.load(Ordering::Acquire),
                ktail_shadow,
                sqe_head: 0,
                sqe_tail: 0,
                ring_ptr,
            }
        }
    }

    #[inline]
    pub fn dropped(&self) -> u32 {
        self.kdropped.load(Ordering::Relaxed)
    }

    #[inline]
    fn vacate_entry(&mut self) -> Option<&mut Entry> {
        if self.sqe_tail.wrapping_sub(self.khead_shadow) == self.kring_entries {
            self.khead_shadow = self.khead.load(Ordering::Acquire);
            if self.sqe_tail.wrapping_sub(self.khead_shadow) == self.kring_entries {
                return None;
            }
        }
        let count = (self.sqe_tail & self.kring_mask) as usize;
        self.sqe_tail = self.sqe_tail.wrapping_add(1);
        let entry = unsafe { &mut *(self.sqes.as_mut_ptr().add(count)) };
        Some(entry)
    }

    #[inline]
    pub(crate) fn prep_rw(
        &mut self,
        opcode: u8,
        fd: RawFd,
        addr: *const libc::c_void,
        len: u32,
        offset: u64,
    ) -> Option<&mut Entry> {
        match self.vacate_entry() {
            Some(sqe) => {
                sqe.opcode = opcode;
                sqe.flags = 0;
                sqe.ioprio = 0;
                sqe.fd = fd;
                sqe.off_addr2 = offset;
                sqe.addr_splice_off_in = addr as u64;
                sqe.len = len;
                sqe.op_flags.rw = 0;
                sqe.user_data = 0;
                sqe.buf_index_group = 0;
                sqe.personality = 0;
                sqe.splice_fd_in = 0;
                sqe._pad2[0] = 0;
                sqe._pad2[1] = 0;
                Some(sqe)
            }
            None => None,
        }
    }

    #[inline]
    pub fn flush(&mut self) -> u32 {
        if self.sqe_head != self.sqe_tail {
            let to_submit = self.sqe_tail.wrapping_sub(self.sqe_head);
            self.sqe_head = self.sqe_tail;

            self.ktail_shadow = self.ktail_shadow.wrapping_add(to_submit);
            self.ktail.store(self.ktail_shadow, Ordering::Release);
        }

        self.khead_shadow = self.khead.load(Ordering::Acquire);
        self.ktail_shadow.wrapping_sub(self.khead_shadow)
    }

    #[inline]
    pub(crate) fn need_wakeup(&self) -> bool {
        (self.kflags.load(Ordering::Relaxed) & Self::NEED_WAKEUP) != 0
    }

    #[inline]
    pub(crate) fn cq_ring_needs_flush(&self) -> bool {
        (self.kflags.load(Ordering::Relaxed) & Self::CQ_OVERFLOW) != 0
    }

    #[inline]
    pub(crate) fn sqes(&self) -> &Mmap<Entry> {
        &self.sqes
    }

    #[inline]
    pub(crate) fn ring_ptr(&self) -> &Mmap<libc::c_void> {
        &self.ring_ptr
    }
}
