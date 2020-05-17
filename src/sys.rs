use core::fmt;
use libc;

use bitflags::bitflags;

#[allow(non_camel_case_types)]
type __kernel_rwf_t = libc::c_int;

#[repr(C)]
#[derive(Copy, Clone)]
pub union OpFlags {
    rw_flags: __kernel_rwf_t,
    fsync_flags: libc::__u32,
    poll_events: libc::__u16,
    sync_range_flags: libc::__u32,
    msg_flags: libc::__u32,
    timeout_flags: libc::__u32,
    accept_flags: libc::__u32,
    cancel_flags: libc::__u32,
    open_flags: libc::__u32,
    statx_flags: libc::__u32,
    fadvise_advice: libc::__u32,
    splice_flags: libc::__u32,
}

impl fmt::Debug for OpFlags {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#x}", unsafe { self.rw_flags })
    }
}

// IO submission data structure (Submission Queue Entry)
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct IoUringSqe {
    opcode: libc::__u8,              // type of operation for this sqe
    flags: libc::__u8,               // IOSQE_ flags
    ioprio: libc::__u16,             // ioprio for the request
    fd: libc::__s32,                 // file descriptor to do IO on
    off_addr2: libc::__u64,          // offset into file
    addr_splice_off_in: libc::__u64, // pointer to buffer or iovecs
    len: libc::__u32,                // buffer size or number of iovecs
    op_flags: OpFlags,
    user_data: libc::__u64, // data to be passed back at completion time

    // [libc::__u64; 3]
    buf_index_group: libc::__u16, // index into fixed buffers, if used; for grouped buffer selection
    personality: libc::__u16,     // personality to use, if used
    splice_fd_in: libc::__s32,
    _pad2: [libc::__u64; 2],
}

bitflags! {
    // sqe -> flags
    struct IoSqeFlags: u8 {
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
    struct IoRingSetupFlags: u32 {
        const IOPOLL    = 1 << 0; // io_context is polled
        const SQPOLL    = 1 << 1; // SQ poll thread
        const SQ_AFF    = 1 << 2; // sq_thread_cpu is valid
        const CQSIZE    = 1 << 3; // app defines CQ size
        const CLAMP     = 1 << 4; // clamp SQ/CQ ring sizes
        const ATTACH_WQ = 1 << 5; // attach to existing wq
    }
}

// #[repr(C)]
// enum IoRingOps {
//      NOP,
//      READV,
//      WRITEV,
//      FSYNC,
//      READ_FIXED,
//      WRITE_FIXED,
//      POLL_ADD,
//      POLL_REMOVE,
//      SYNC_FILE_RANGE,
//      SENDMSG,
//      RECVMSG,
//      TIMEOUT,
//      TIMEOUT_REMOVE,
//      ACCEPT,
//      ASYNC_CANCEL,
//      LINK_TIMEOUT,
//      CONNECT,
//      FALLOCATE,
//      OPENAT,
//      CLOSE,
//      FILES_UPDATE,
//      STATX,
//      READ,
//      WRITE,
//      FADVISE,
//      MADVISE,
//      SEND,
//      RECV,
//      OPENAT2,
//      EPOLL_CTL,
//      SPLICE,
//      PROVIDE_BUFFERS,
//      REMOVE_BUFFERS,

//      // this goes last, obviously
//      LAST,
//  }

//  /*
//   * sqe->fsync_flags
//   */
//  #define IORING_FSYNC_DATASYNC	(1U << 0)

//  /*
//   * sqe->timeout_flags
//   */
//  #define IORING_TIMEOUT_ABS	(1U << 0)

//  /*
//   * sqe->splice_flags
//   * extends splice(2) flags
//   */
//  #define SPLICE_F_FD_IN_FIXED	(1U << 31) /* the last bit of __u32 */
//  /*
//   * IO completion data structure (Completion Queue Entry)
//   */
//  struct io_uring_cqe {
//      __u64	user_data;	/* sqe->data submission passed back */
//      __s32	res;		/* result code for this event */
//      __u32	flags;
//  };

//  /*
//   * cqe->flags
//   *
//   * IORING_CQE_F_BUFFER	If set, the upper 16 bits are the buffer ID
//   */
//  #define IORING_CQE_F_BUFFER		(1U << 0)

//  enum {
//      IORING_CQE_BUFFER_SHIFT		= 16,
//  };

//  /*
//   * Magic offsets for the application to mmap the data it needs
//   */
//  #define IORING_OFF_SQ_RING		0ULL
//  #define IORING_OFF_CQ_RING		0x8000000ULL
//  #define IORING_OFF_SQES			0x10000000ULL

//  /*
//   * Filled with the offset for mmap(2)
//   */
//  struct io_sqring_offsets {
//      __u32 head;
//      __u32 tail;
//      __u32 ring_mask;
//      __u32 ring_entries;
//      __u32 flags;
//      __u32 dropped;
//      __u32 array;
//      __u32 resv1;
//      __u64 resv2;
//  };

//  /*
//   * sq_ring->flags
//   */
//  #define IORING_SQ_NEED_WAKEUP	(1U << 0) /* needs io_uring_enter wakeup */
//  struct io_cqring_offsets {
//      __u32 head;
//      __u32 tail;
//      __u32 ring_mask;
//      __u32 ring_entries;
//      __u32 overflow;
//      __u32 cqes;
//      __u64 resv[2];
//  };

//  /*
//   * io_uring_enter(2) flags
//   */
//  #define IORING_ENTER_GETEVENTS	(1U << 0)
//  #define IORING_ENTER_SQ_WAKEUP	(1U << 1)

//  /*
//   * Passed in for io_uring_setup(2). Copied back with updated info on success
//   */
//  struct io_uring_params {
//      __u32 sq_entries;
//      __u32 cq_entries;
//      __u32 flags;
//      __u32 sq_thread_cpu;
//      __u32 sq_thread_idle;
//      __u32 features;
//      __u32 wq_fd;
//      __u32 resv[3];
//      struct io_sqring_offsets sq_off;
//      struct io_cqring_offsets cq_off;
//  };

//  /*
//   * io_uring_params->features flags
//   */
//  #define IORING_FEAT_SINGLE_MMAP		(1U << 0)
//  #define IORING_FEAT_NODROP		(1U << 1)
//  #define IORING_FEAT_SUBMIT_STABLE	(1U << 2)
//  #define IORING_FEAT_RW_CUR_POS		(1U << 3)
//  #define IORING_FEAT_CUR_PERSONALITY	(1U << 4)
//  #define IORING_FEAT_FAST_POLL		(1U << 5)

//  /*
//   * io_uring_register(2) opcodes and arguments
//   */
//  #define IORING_REGISTER_BUFFERS		0
//  #define IORING_UNREGISTER_BUFFERS	1
//  #define IORING_REGISTER_FILES		2
//  #define IORING_UNREGISTER_FILES		3
//  #define IORING_REGISTER_EVENTFD		4
//  #define IORING_UNREGISTER_EVENTFD	5
//  #define IORING_REGISTER_FILES_UPDATE	6
//  #define IORING_REGISTER_EVENTFD_ASYNC	7
//  #define IORING_REGISTER_PROBE		8
//  #define IORING_REGISTER_PERSONALITY	9
//  #define IORING_UNREGISTER_PERSONALITY	10

//  struct io_uring_files_update {
//      __u32 offset;
//      __u32 resv;
//      __aligned_u64 /* __s32 * */ fds;
//  };

//  #define IO_URING_OP_SUPPORTED	(1U << 0)

//  struct io_uring_probe_op {
//      __u8 op;
//      __u8 resv;
//      __u16 flags;	/* IO_URING_OP_* flags */
//      __u32 resv2;
//  };

//  struct io_uring_probe {
//      __u8 last_op;	/* last opcode supported */
//      __u8 ops_len;	/* length of ops[] array below */
//      __u16 resv;
//      __u32 resv2[3];
//      struct io_uring_probe_op ops[0];
//  };

//  #endif
