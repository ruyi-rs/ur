use std::mem::MaybeUninit;


// IO completion data structure (Completion Queue Entry)
// struct io_uring_cqe
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Entry {
    user_data: u64, // sqe->data submission passed back
    res: i32,       // result code for this event
    flags: u32,     // IoRingCqeFlags::* flags
}

#[derive(Debug, Copy, Clone)]
pub(crate) struct Queue {}

impl Queue {
    #[inline]
    pub fn new() -> Self {
        unsafe { MaybeUninit::zeroed().assume_init() }
    }
}
