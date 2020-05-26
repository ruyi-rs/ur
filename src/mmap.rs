use std::io::Result;
use std::ptr;

use libc;

use crate::sys;

#[derive(Debug)]
pub struct Pointer {
    addr: ptr::NonNull<libc::c_void>,
    len: usize,
}

impl Pointer {
    #[inline]
    pub fn try_new(len: usize, fd: i32, offset: i64) -> Result<Self> {
        let addr = unsafe {
            let ptr = sys::mmap(
                ptr::null_mut(),
                len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED | libc::MAP_POPULATE,
                fd,
                offset,
            )?;
            ptr::NonNull::new_unchecked(ptr)
        };
        Ok(Self { addr, len })
    }
}

impl Drop for Pointer {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            sys::munmap(self.addr.as_mut(), self.len).ok();
        }
    }
}
