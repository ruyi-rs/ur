use std::io::{Error, Result};
use std::marker::PhantomData;
use std::os::unix::io::RawFd;
use std::{fmt, mem};

use bitflags::bitflags;
use libc;

use crate::params;

#[allow(non_camel_case_types)]
pub type __kernel_rwf_t = libc::c_int;

#[repr(C)]
#[derive(Copy, Clone)]
pub union OpFlags {
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
#[derive(Debug, Copy, Clone)]
pub struct IoUringSqe {
    pub opcode: libc::__u8,              // type of operation for this sqe
    pub flags: libc::__u8,               // IOSQE_ flags (IoSqe::*)
    pub ioprio: libc::__u16,             // ioprio for the request
    pub fd: libc::__s32,                 // file descriptor to do IO on
    pub off_addr2: libc::__u64,          // offset into file
    pub addr_splice_off_in: libc::__u64, // pointer to buffer or iovecs
    pub len: libc::__u32,                // buffer size or number of iovecs
    pub op_flags: OpFlags,
    pub user_data: libc::__u64, // data to be passed back at completion time

    // [libc::__u64; 3]
    pub buf_index_group: libc::__u16, // index into fixed buffers, if used; for grouped buffer selection
    pub personality: libc::__u16,     // personality to use, if used
    pub splice_fd_in: libc::__s32,
    _pad2: [libc::__u64; 2],
}

bitflags! {
    // sqe -> flags
    // IOSQE_ flags
    pub struct IoSqe: libc::__u8 {
        const FIXED_FILE    = 1 << 0; // use fixed fileset
        const IO_DRAIN      = 1 << 1; // issue after inflight IO
        const IO_LINK       = 1 << 2; // links next sqe
        const IO_HARDLINK   = 1 << 3; // like LINK, but stronger
        const ASYNC         = 1 << 4; // always go async
        const BUFFER_SELECT = 1 << 5; // select buffer from sqe->buf_group
    }
}

bitflags! {
    // io_uring_setup() flags
    // IORING_SETUP_ flags
    pub struct IoRingSetup: libc::__u32 {
        const IOPOLL    = 1 << 0; // io_context is polled
        const SQPOLL    = 1 << 1; // SQ poll thread
        const SQ_AFF    = 1 << 2; // sq_thread_cpu is valid
        const CQSIZE    = 1 << 3; // app defines CQ size
        const CLAMP     = 1 << 4; // clamp SQ/CQ ring sizes
        const ATTACH_WQ = 1 << 5; // attach to existing wq
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub enum IoRingOp {
    NOP,
    READV,
    WRITEV,
    FSYNC,
    READ_FIXED,
    WRITE_FIXED,
    POLL_ADD,
    POLL_REMOVE,
    SYNC_FILE_RANGE,
    SENDMSG,
    RECVMSG,
    TIMEOUT,
    TIMEOUT_REMOVE,
    ACCEPT,
    ASYNC_CANCEL,
    LINK_TIMEOUT,
    CONNECT,
    FALLOCATE,
    OPENAT,
    CLOSE,
    FILES_UPDATE,
    STATX,
    READ,
    WRITE,
    FADVISE,
    MADVISE,
    SEND,
    RECV,
    OPENAT2,
    EPOLL_CTL,
    SPLICE,
    PROVIDE_BUFFERS,
    REMOVE_BUFFERS,

    // this goes last, obviously
    LAST,
}

// sqe->fsync_flags
bitflags! {
    pub struct IoRingFsync: libc::__u32 {
        const DATASYNC = 1 << 0;
    }
}

// sqe->timeout_flags
bitflags! {
    pub struct IoRingTimeout: libc::__u32 {
        const ABS = 1 << 0;
    }
}

// sqe->splice_flags
// extends splice(2) flags
bitflags! {
    pub struct SpliceFlags: libc::__u32 {
        const FD_IN_FIXED = 1 << 31; // the last bit of __u32
    }
}

// IO completion data structure (Completion Queue Entry)
// struct io_uring_cqe
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct IoUringCqe {
    pub user_data: libc::__u64, // sqe->data submission passed back
    pub res: libc::__s32,       // result code for this event
    pub flags: libc::__u32,     // IoRingCqeFlags::* flags
}

// cqe->flags
bitflags! {
    pub struct IoRingCqeFlags: libc::__u32 {
        const BUFFER = 1 << 0; // If set, the upper 16 bits are the buffer ID
    }
}

pub const IORING_CQE_BUFFER_SHIFT: usize = 16;

// Magic offsets for the application to mmap the data it needs
pub const IORING_OFF_SQ_RING: usize = 0;
pub const IORING_OFF_CQ_RING: usize = 0x800_0000;
pub const IORING_OFF_SQES: usize = 0x1000_0000;

// Filled with the offset for mmap(2)
// struct io_sqring_offsets
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct IoSqringOffsets {
    pub head: libc::__u32,
    pub tail: libc::__u32,
    pub ring_mask: libc::__u32,
    pub ring_entries: libc::__u32,
    pub flags: libc::__u32, // IoRingSq::* flags
    pub dropped: libc::__u32,
    pub array: libc::__u32,
    _resv1: libc::__u32,
    _resv2: libc::__u64,
}

// sq_ring->flags
bitflags! {
    pub struct IoRingSq: libc::__u32 {
        const NEED_WAKEUP = 1 << 0; // needs io_uring_enter wakeup
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
// struct io_cqring_offsets
pub struct IoCqringOffsets {
    pub head: libc::__u32,
    pub tail: libc::__u32,
    pub ring_mask: libc::__u32,
    pub ring_entries: libc::__u32,
    pub overflow: libc::__u32,
    pub cqes: libc::__u32,
    _resv: libc::__u64,
}

// io_uring_enter(2) flags
bitflags! {
    pub struct IoRingEnter: libc::c_uint {
        const GETEVENTS = 1 << 0;
        const SQ_WAKEUP = 1 << 1;
    }
}

// Passed in for io_uring_setup(2). Copied back with updated info on success
// struct io_uring_params
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct IoUringParams {
    pub sq_entries: libc::__u32,
    pub cq_entries: libc::__u32,
    pub flags: libc::__u32, // IORING_SETUP_ flags (IoRingSetup::*)
    pub sq_thread_cpu: libc::__u32,
    pub sq_thread_idle: libc::__u32,
    pub features: libc::__u32, // IoRingFeat::* flags
    pub wq_fd: libc::__u32,
    _resv: [libc::__u32; 3],
    pub sq_off: IoSqringOffsets,
    pub cq_off: IoCqringOffsets,
}

// io_uring_params->features flags
bitflags! {
    pub struct IoRingFeat: libc::__u32 {
        const SINGLE_MMAP       = 1 << 0;
        const NODROP            = 1 << 1;
        const SUBMIT_STABLE     = 1 << 2;
        const RW_CUR_POS        = 1 << 3;
        const CUR_PERSONALITY   = 1 << 4;
        const FAST_POLL         = 1 << 5;
    }
}

// io_uring_register(2) opcodes and arguments
pub const IORING_REGISTER_BUFFERS: libc::c_uint = 0;
pub const IORING_UNREGISTER_BUFFERS: libc::c_uint = 1;
pub const IORING_REGISTER_FILES: libc::c_uint = 2;
pub const IORING_UNREGISTER_FILES: libc::c_uint = 3;
pub const IORING_REGISTER_EVENTFD: libc::c_uint = 4;
pub const IORING_UNREGISTER_EVENTFD: libc::c_uint = 5;
pub const IORING_REGISTER_FILES_UPDATE: libc::c_uint = 6;
pub const IORING_REGISTER_EVENTFD_ASYNC: libc::c_uint = 7;
pub const IORING_REGISTER_PROBE: libc::c_uint = 8;
pub const IORING_REGISTER_PERSONALITY: libc::c_uint = 9;
pub const IORING_UNREGISTER_PERSONALITY: libc::c_uint = 10;

// struct io_uring_files_update
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct IoUringFilesUpdate {
    pub offset: libc::__u32,
    pub _resv: libc::__u32,
    pub fds: libc::__u64,
}

// IO_URING_OP_ flags
bitflags! {
    pub struct IoUringOp: libc::__u8 {
        const SUPPORTED = 1 << 0;
    }
}

// struct io_uring_probe_op
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct IoUringProbeOp {
    pub op: libc::__u8,
    _resv: libc::__u8,
    pub flags: libc::__u16, // IO_URING_OP_* flags (IoUringOp::*)
    _resv2: libc::__u32,
}

// struct io_uring_probe
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct IoUringProbe {
    pub last_op: libc::__u8,
    pub ops_len: libc::__u8,
    _resv: libc::__u16,
    _resv2: libc::__u32,
    _ops: PhantomData<[IoUringProbeOp; 0]>,
}

#[inline]
unsafe fn io_uring_register(
    fd: libc::c_int,
    opcode: libc::c_uint,
    arg: *const libc::c_void,
    nr_args: libc::c_uint,
) -> libc::c_int {
    #[allow(non_upper_case_globals)]
    const __NR_io_uring_register: libc::c_long = 425;

    libc::syscall(__NR_io_uring_register, fd, opcode, arg, nr_args) as libc::c_int
}

#[inline]
pub fn io_uring_setup(entries: u32, p: &mut params::IoUringParams) -> libc::c_int {
    #[allow(non_upper_case_globals)]
    const __NR_io_uring_setup: libc::c_long = 426;

    libc::syscall(__NR_io_uring_setup, entries as libc::c_uint, p) as libc::c_int
}

#[inline]
unsafe fn io_uring_enter(
    fd: libc::c_int,
    to_submit: libc::c_uint,
    min_complete: libc::c_uint,
    flags: libc::c_uint,
    sig: *const libc::sigset_t,
) -> libc::c_int {
    #[allow(non_upper_case_globals)]
    const __NR_io_uring_enter: libc::c_long = 427;

    libc::syscall(
        __NR_io_uring_enter,
        fd,
        to_submit,
        min_complete,
        flags,
        sig,
        mem::size_of::<libc::sigset_t>(),
    ) as libc::c_int
}
