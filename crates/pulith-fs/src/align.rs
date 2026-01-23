use std::alloc::{alloc, dealloc, Layout};
use std::marker::PhantomData;

pub const PAGE_SIZE: usize = 4096;

#[derive(Debug)]
pub struct AlignedBuf {
    ptr: *mut u8,
    layout: Layout,
    _marker: PhantomData<[u8]>,
}

impl AlignedBuf {
    pub fn new(size: usize, align: usize) -> Result<Self, std::io::Error> {
        let layout = Layout::from_size_align(size, align)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

        let ptr = unsafe { alloc(layout) };
        if ptr.is_null() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::OutOfMemory,
                "allocation failed",
            ));
        }

        Ok(Self {
            ptr,
            layout,
            _marker: PhantomData,
        })
    }

    pub fn new_page_aligned(size: usize) -> Result<Self, std::io::Error> {
        Self::new(size, PAGE_SIZE)
    }

    pub fn from_slice(data: &[u8], align: usize) -> Result<Self, std::io::Error> {
        let mut buf = Self::new(data.len(), align)?;
        buf.as_mut_slice().copy_from_slice(data);
        Ok(buf)
    }

    pub fn from_slice_page_aligned(data: &[u8]) -> Result<Self, std::io::Error> {
        Self::from_slice(data, PAGE_SIZE)
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.layout.size()) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.layout.size()) }
    }

    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.ptr
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.ptr
    }

    pub fn len(&self) -> usize {
        self.layout.size()
    }

    pub fn is_empty(&self) -> bool {
        self.layout.size() == 0
    }
}

impl Drop for AlignedBuf {
    fn drop(&mut self) {
        unsafe { dealloc(self.ptr, self.layout) }
    }
}

unsafe impl Send for AlignedBuf {}
unsafe impl Sync for AlignedBuf {}

pub fn align_down(n: usize, align: usize) -> usize {
    n & !(align - 1)
}

pub fn align_up(n: usize, align: usize) -> usize {
    (n + align - 1) & !(align - 1)
}

pub fn is_aligned(n: usize, align: usize) -> bool {
    n & (align - 1) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aligned_buf() {
        let mut buf = AlignedBuf::new(8192, 4096).unwrap();
        assert_eq!(buf.len(), 8192);
        buf.as_mut_slice()[0] = 42;
        assert_eq!(buf.as_slice()[0], 42);
    }

    #[test]
    fn test_from_slice() {
        let data = b"hello world";
        let buf = AlignedBuf::from_slice(data, 4096).unwrap();
        assert_eq!(buf.as_slice(), data);
    }

    #[test]
    fn test_align_functions() {
        assert_eq!(align_down(100, 16), 96);
        assert_eq!(align_up(100, 16), 112);
        assert!(is_aligned(16, 16));
        assert!(!is_aligned(17, 16));
    }
}
