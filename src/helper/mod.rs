use tokio_uring::buf::IoBuf;


/// unsafe BytesSlice Wrapper for Sending bytes safely to tokio uring api
pub struct BytesSliceWrapper {
    bytes:*const u8,
    len:usize,
}

impl BytesSliceWrapper {

    /// creating new BytesSliceWrapper from ref [[u8]]
    pub fn new(bytes:&[u8])->BytesSliceWrapper{
        let d = unsafe {bytes.as_ptr()};
        BytesSliceWrapper {
            bytes:d,
            len:bytes.len()
        }
    }
}

impl Into<BytesSliceWrapper> for &'_ [u8]{
    fn into(self) -> BytesSliceWrapper {
        BytesSliceWrapper::new(self)
    }
}

unsafe impl IoBuf for BytesSliceWrapper {
    fn stable_ptr(&self) -> *const u8 {
        self.bytes
    }

    fn bytes_init(&self) -> usize {
        self.len
    }

    fn bytes_total(&self) -> usize {
        self.len
    }
}