use std::mem::MaybeUninit;


// IO completion data structure (Completion Queue Entry)
// struct io_uring_cqe
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Entry {
    pub user_data: u64, // sqe->data submission passed back
    pub res: i32,       // result code for this event
    pub flags: u32,     // IoRingCqeFlags::* flags
}

#[derive(Debug, Copy, Clone)]
pub struct Queue {}

impl Queue {
    #[inline]
    pub fn new() -> Self {
        unsafe { MaybeUninit::zeroed().assume_init() }
    }
}
