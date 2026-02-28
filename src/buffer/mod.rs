
//! # WaterBuffer
//!
//! `WaterBuffer` is a generic dynamically-sized buffer in Rust, primarily for bytes (`u8`).
//! It provides safe memory management, dynamic resizing, iteration, and indexing.

use std::alloc::{alloc, dealloc, realloc, Layout};
use std::mem::MaybeUninit;
use std::ops::{Deref, DerefMut, Index, IndexMut, Range, RangeFrom, RangeFull, RangeTo};
use std::ptr;
#[cfg(feature = "bytes")]
use bytes::buf::UninitSlice;
#[cfg(feature = "bytes")]
use bytes::BufMut;
#[cfg(feature = "uring")]
use tokio_uring::buf::{BoundedBuf, BoundedBufMut, IoBuf, IoBufMut};


type InnerType = u8;

#[derive(Debug)]
/// Main dynamic buffer struct
pub struct WaterBuffer<T> {
    pub (crate) cap: usize,
    pub(crate) start_pos: usize,
    #[cfg(feature = "circular_buffer")]
    pub (crate) circular_position:Option<usize>,
    pub (crate) pointer: *mut T,
    #[cfg(feature = "unsafe_clone")]
    pub (crate) original:Option<*mut WaterBuffer<T>>,
    pub (crate) filled_data_length
    : usize,
}


#[cfg(feature = "unsafe_clone")]
impl  WaterBuffer<InnerType> {
    /// it's returning the same class but without destructing data ,so you need to keep the real original struct alive
    /// and never insert new data through the new one
    pub unsafe fn unsafe_clone(&self) -> Self {
        let original = match self.original {
            None => {self as *const WaterBuffer<InnerType> as  *mut WaterBuffer<InnerType>}
            Some(e) => {e}
        };
        return  WaterBuffer {
            cap:self.cap,
            pointer:self.pointer,
            start_pos:self.start_pos,
            filled_data_length:self.filled_data_length,
            original:Some(original)
        }
    }
}

unsafe impl Send for WaterBuffer<*mut u8> {}
unsafe impl Send for WaterBuffer<u8> {}


impl WaterBuffer<InnerType> {
    /// Converts the buffer into an owned iterator
    pub fn into_owned_iter(self) -> WaterBufferOwnedIter<InnerType> {
        self.into()
    }

    /// Returns an immutable iterator over the buffer
    pub fn iter(&self) -> WaterBufferIter<'_> {
        WaterBufferIter {
            buffer: self,
            pos: 0,
        }
    }


    pub fn spare_capacity_mut(&mut self)->&mut [MaybeUninit<u8>]{
        unsafe {
            let pos = self.start_pos + self.filled_data_length;
            let pointer = self.pointer.add(pos);
            std::slice::from_raw_parts_mut(pointer as *mut MaybeUninit<u8>, self.cap - pos)
        }
    }

    /// Returns a mutable iterator over the buffer
    pub fn iter_mut(&mut self) -> WaterBufferIterMut<'_> {
        WaterBufferIterMut {
            buffer: self,
            pos: 0,
        }
    }



    #[inline(always)]
    pub const fn capacity(&self)->usize{
        self.cap - self.filled_data_length
    }

    /// Creates a new buffer with a given capacity
    pub fn with_capacity(cap: usize) -> WaterBuffer<InnerType> {
        let layout = Layout::array::<InnerType>(cap).unwrap();
        let first_element_pointer = unsafe { alloc(layout) } as *mut InnerType;
        #[cfg(feature = "circular_buffer")]
        {
            return  WaterBuffer {
                cap,
                pointer: first_element_pointer,
                start_pos: 0,
                filled_data_length
                : 0,
                circular_position:None
            };
        };

        #[cfg(all(not(feature = "circular_buffer")))]
        {
            #[cfg(feature = "unsafe_clone")]
            {
                return   WaterBuffer {
                    cap,
                    pointer: first_element_pointer,
                    start_pos: 0,
                    filled_data_length
                    : 0,
                    original:None
                }
            }
            #[cfg(not(feature = "unsafe_clone"))]
            {
                return   WaterBuffer {
                    cap,
                    pointer: first_element_pointer,
                    start_pos: 0,
                    filled_data_length
                    : 0,
                }
            }
        }
    }

    /// Expands the buffer to a new capacity
    #[inline(always)]
    pub fn expand(&mut self,mut n: usize) {
        n += self.cap;
        let new_pointer = unsafe {
            realloc(
                self.pointer as *mut u8,
                Layout::array::<InnerType>(self.cap).unwrap(),
                n,
            )
        } as *mut InnerType;
        self.pointer = new_pointer;
        self.cap = n;
    }

    #[cfg(not(feature = "circular_buffer"))]
    /// Calculates an appropriate size for buffer growth
    #[inline(always)]
    pub(crate) const fn ap_size(&self, additional: usize) -> usize {
        if self.cap == 0 {
            if additional < 64 {return 64}
            return additional;
        }
        let needed = self.filled_data_length + additional;
        let cap = self.cap * 2 ;
        if cap > needed { return cap}
        needed
    }

    #[cfg(feature = "circular_buffer")]
    /// Extends the buffer from a slice
    #[inline(always)]
    pub fn extend_from_slice(&mut self,mut slice:&[u8]){
        if self.cap == 0 {return;}
        let mut must_write_len = slice.len();
        while must_write_len > 0 {

            let  mut position_to_write = self.circular_position.unwrap_or_else(||{
                let filled = self.filled_data_length + self.start_pos;
                if filled >= self.cap {
                    self.circular_position = Some(0);
                    return 0
                }
                filled
            });
            if position_to_write >= self.cap {
                position_to_write = 0;
            }
            let available_len = (self.cap - position_to_write).min(must_write_len);
            let n_slice = &slice[..available_len];
            unsafe {
                ptr::copy_nonoverlapping(
                    n_slice.as_ptr(),
                    self.pointer.add(position_to_write),
                    n_slice.len()
                );
            };

            if self.filled_data_length < self.cap {
                self.filled_data_length += available_len;
            } else {
                if let Some(cp) = self.circular_position.as_mut() {
                    *cp += available_len;
                    if *cp >= self.cap {
                        *cp = 0;
                    }
                } else {
                    let mut p = position_to_write+available_len;
                    if p >= self.cap {
                        p = 0 ;
                    }
                    self.circular_position = Some(p);
                }
            }
            must_write_len -= available_len;
            slice = &slice[available_len..];
        }
        // let`s update filled_data
        self.filled_data_length += slice.len();
        if self.filled_data_length >= self.cap {
            self.filled_data_length = self.cap;
        }
    }
    // pub fn extend_from_slice(&mut self, slice: &[u8]) {
    //     if slice.is_empty() {
    //         return;
    //     }
    //
    //     let slice_len = slice.len();
    //     let current_len = self.len();
    //
    //     // If incoming data is larger than capacity, only keep the last `cap` bytes
    //     if slice_len >= self.cap {
    //         let offset = slice_len - self.cap;
    //         unsafe {
    //             ptr::copy_nonoverlapping(
    //                 slice.as_ptr().add(offset),
    //                 self.pointer,
    //                 self.cap,
    //             );
    //         }
    //         self.filled_data_length = self.cap;
    //         self.circular_position = Some(0);
    //         self.start_pos = 0;
    //         return;
    //     }
    //
    //     // Determine write position
    //     let write_pos = if let Some(circ_pos) = self.circular_position {
    //         circ_pos
    //     } else if self.filled_data_length < self.cap {
    //         self.filled_data_length
    //     } else {
    //         // First wrap - should not happen but handle it
    //         self.circular_position = Some(0);
    //         0
    //     };
    //
    //     // Calculate how much space until wrap
    //     let space_until_wrap = self.cap - write_pos;
    //
    //     if slice_len <= space_until_wrap {
    //         // Simple case: no wrap needed, write everything at once
    //         unsafe {
    //             ptr::copy_nonoverlapping(
    //                 slice.as_ptr(),
    //                 self.pointer.add(write_pos),
    //                 slice_len,
    //             );
    //         }
    //
    //         let new_pos = write_pos + slice_len;
    //         let new_filled = current_len + slice_len;
    //
    //         if new_filled >= self.cap {
    //             self.filled_data_length = self.cap;
    //             self.circular_position = Some(if new_pos >= self.cap { 0 } else { new_pos });
    //         } else {
    //             self.filled_data_length = new_filled;
    //             if self.circular_position.is_some() {
    //                 self.circular_position = Some(new_pos);
    //             }
    //         }
    //     } else {
    //         // Complex case: need to wrap
    //
    //         // Write first chunk (until end of buffer)
    //         unsafe {
    //             ptr::copy_nonoverlapping(
    //                 slice.as_ptr(),
    //                 self.pointer.add(write_pos),
    //                 space_until_wrap,
    //             );
    //         }
    //
    //         // Write second chunk (from beginning of buffer)
    //         let remaining = slice_len - space_until_wrap;
    //         unsafe {
    //             ptr::copy_nonoverlapping(
    //                 slice.as_ptr().add(space_until_wrap),
    //                 self.pointer,
    //                 remaining,
    //             );
    //         }
    //
    //         // Update state
    //         self.filled_data_length = self.cap;
    //         self.circular_position = Some(remaining);
    //     }
    // }



    #[inline(always)]

     const fn shift_data(&mut self){
        unsafe{
            ptr::copy_nonoverlapping(
                self.pointer.add(self.start_pos),
                self.pointer,
                self.filled_data_length
            );
        }
        self.start_pos = 0;
    }

    #[inline]
    pub const fn available(&self)->usize{
        self.cap - self.filled_data_length
    }


    #[inline(always)]
    pub fn reserve(&mut self,len:usize){
        if self.is_empty() {
            self.clear();
        }
        if len < self.remaining_mut() {
            return
        }
        let raw_available = self.cap - (self.start_pos + self.filled_data_length);
        if raw_available < len {
            if  self.start_pos >= self.filled_data_length && self.available() >= len {
                self.shift_data();
            } else {
                self.expand(self.ap_size(len));
            }
        }
    }
    #[cfg(not(feature = "circular_buffer"))]
    /// Extends the buffer from a slice
    #[inline(always)]
    pub fn extend_from_slice(&mut self, slice: &[u8]) {
        let len  = slice.len();
        self.reserve(len);
        unsafe {
            ptr::copy_nonoverlapping(
                slice.as_ptr(),
                self.pointer.add(self.start_pos + self.filled_data_length) as *mut u8,
                len,
            )
        };
        self.filled_data_length += len;
    }

    /// Returns the number of elements in the buffer
    #[inline(always)]
    pub const fn len(&self) -> usize {
        #[cfg(feature = "circular_buffer")]
        {
            if self.filled_data_length >= self.cap {

                return self.cap ;
            }
        }
        self.filled_data_length

    }


    #[cfg(feature = "circular_buffer")]
    #[inline(always)]
    pub const fn reset(&mut self){
        self.start_pos = 0;
        self.filled_data_length = 0;
        self.circular_position = None;
    }
    #[cfg(not(feature = "circular_buffer"))]
    /// Resets the buffer
    #[inline(always)]
    pub const fn reset(&mut self) {
        self.filled_data_length = 0;
        self.start_pos = 0;
    }



    #[inline]
    #[cfg(feature = "circular_buffer")]
    pub fn push(&mut self, item: InnerType) {
        if self.filled_data_length >= self.cap {
            // let mut p = self.circular_position.unwrap_or(self.start_pos);
            let  p = self.circular_position.as_ref().unwrap_or(&0);
            unsafe {
                ptr::copy_nonoverlapping(
                    &item,
                    self.pointer.add(*p),
                    1,
                );
            }
            let n = *p + 1;
            self.circular_position = Some(
                if n >= self.cap {
                    0
                } else { n }
            );
            return
        }
        unsafe {
            ptr::copy_nonoverlapping(
                &item,
                self.pointer.add(self.filled_data_length),
                1,
            );
        }
        self.filled_data_length += 1;
    }



    #[cfg(not(feature = "circular_buffer"))]
    #[inline(always)]
    pub fn push(&mut self, item: InnerType) {
        // Check if we need more space at the end
        if self.start_pos + self.filled_data_length >= self.cap {
            if self.start_pos > 0 {
                // Compact: move data to beginning
                self.shift_data();
            } else {
                // Expand: need more capacity
                let growth = if self.cap == 0 { 64 } else { self.cap };
                self.expand(growth);
            }
        }

        unsafe {
            *self.pointer.add(self.start_pos + self.filled_data_length) = item;
        }
        self.filled_data_length += 1;
    }

    #[inline(always)]
    pub const fn clear(&mut self) {
        self.reset();
    }



    #[inline(always)]
    pub const fn advance(&mut self, n: usize) {
        if n > self.filled_data_length
        {
            panic!("Insufficient space to advance");
        }
        self.start_pos += n;
        self.filled_data_length -= n;
    }

    #[inline(always)]
    pub const fn remaining(&self) -> usize {
        self.len()
    }

    #[cfg(feature = "circular_buffer")]
    #[inline(always)]
    pub const fn un_initialized_remaining(&self) -> usize {
        let mut pos = self.filled_data_length;
        if let Some(p) = self.circular_position {
            if p > 0 {pos = p;}
        }
        if pos > self.cap { return  0;}
        self.cap - pos

    }
    #[cfg(not(feature = "circular_buffer"))]
    #[inline(always)]
    pub const fn un_initialized_remaining(&self) -> usize {
        self.cap - self.filled_data_length

    }


    #[cfg(not(feature = "circular_buffer"))]

    #[inline(always)]
    pub const fn chunk_mut(&mut self) -> &mut [u8] {
        unsafe {
            let pos = self.start_pos + self.filled_data_length;
            let pointer = self.pointer.add(pos);
            std::slice::from_raw_parts_mut(pointer, self.cap - pos)
        }
    }
    #[cfg(not(feature = "circular_buffer"))]

    #[inline(always)]
    pub const fn chunk(&self) -> &[u8] {
        unsafe {
            let pos = self.start_pos ;
            let pointer = self.pointer.add(pos);
            std::slice::from_raw_parts(pointer, self.filled_data_length)
        }
    }

    #[cfg(feature = "circular_buffer")]
    #[inline(always)]
    pub const fn chunk_mut(&mut self) -> &mut [u8] {
        unsafe {
            let pos = self.start_pos + self.filled_data_length;
            if pos >= self.cap {
                return  &mut [];
            }
            let pointer = self.pointer.add(pos);
            std::slice::from_raw_parts_mut(pointer, self.cap - pos)
        }
    }
    #[inline(always)]
    pub const fn advance_mut(&mut self, n: usize) {
        self.filled_data_length
            += n;
    }



}

#[cfg(feature = "bytes")]
unsafe impl  BufMut for WaterBuffer<InnerType>{
    #[inline]
    fn remaining_mut(&self) -> usize {
        // How many bytes can still be written
        self.un_initialized_remaining()
    }

    #[inline]
    unsafe fn advance_mut(&mut self, cnt: usize) {
        // SAFETY: caller guarantees cnt <= remaining_mut()
        self.filled_data_length += cnt;
    }

    #[inline]
    fn chunk_mut(&mut self) -> &mut UninitSlice {
        // Pointer to where new data should be written
        let write_pos = self.start_pos + self.filled_data_length;

        if write_pos >= self.cap {
            return UninitSlice::new(&mut []);
        }

        let len = self.cap - write_pos;

        unsafe {
            let ptr = self.pointer.add(write_pos);
            UninitSlice::from_raw_parts_mut(ptr, len)
        }
    }
}
impl Into<WaterBufferOwnedIter<InnerType>> for WaterBuffer<InnerType> {
    fn into(self) -> WaterBufferOwnedIter<InnerType> {
        WaterBufferOwnedIter {
            buffer: self,
            iterator_pos: 0,
        }
    }
}

/// Drop implementation to free memory
impl<T> Drop for WaterBuffer<T> {
    fn drop(&mut self) {
       #[cfg(feature = "unsafe_clone")]
       {
           match self.original {
               None => {}
               Some(e) => {
                   let e = unsafe {&mut *e};
                   e.cap = self.cap;
                   e.pointer = self.pointer;
                   e.start_pos = self.start_pos;
                   e.filled_data_length = self.filled_data_length;
                   return
               }
           }
       }
        if !self.pointer.is_null() && self.cap > 0 {
            let layout = Layout::array::<T>(self.cap).unwrap();
            unsafe {
                dealloc(self.pointer as *mut u8, layout);
            }
        }
    }
}

/// Owned iterator over `WaterBuffer`
pub struct WaterBufferOwnedIter<InnerType> {
    iterator_pos: usize,
    buffer: WaterBuffer<InnerType>,
}

impl Iterator for WaterBufferOwnedIter<InnerType> {
    type Item = InnerType;


    fn next(&mut self) -> Option<Self::Item> {
        if self.iterator_pos >= self.buffer.cap.min(self.buffer.filled_data_length)
        {
            return None;
        }
        #[cfg(not(feature = "circular_buffer"))]
        if self.iterator_pos >= self.buffer.filled_data_length {
            return None;
        }

        let item = unsafe { &*self.buffer.pointer.add(self.iterator_pos + self.buffer.start_pos) };
        self.iterator_pos += 1;
        Some(*item)
    }
}

/// Immutable iterator
pub struct WaterBufferIter<'a> {
    buffer: &'a WaterBuffer<u8>,
    pos: usize,
}

/// Mutable iterator
pub struct WaterBufferIterMut<'a> {
    buffer: &'a mut WaterBuffer<u8>,
    pos: usize,
}

impl<'a> Iterator for WaterBufferIter<'a> {
    type Item = &'a u8;

    fn next(&mut self) -> Option<Self::Item> {
        #[cfg(feature = "circular_buffer")]
        if self.pos >= self.buffer.cap.min(self.buffer.filled_data_length)
        {
            return None;
        }
        #[cfg(not(feature = "circular_buffer"))]
        if self.pos >= self.buffer.filled_data_length {
            return None;
        }

        let item = unsafe { &*self.buffer.pointer.add(self.pos + self.buffer.start_pos) };
        self.pos += 1;
        Some(item)
    }
}

impl<'a> Iterator for WaterBufferIterMut<'a> {
    type Item = &'a mut u8;

    fn next(&mut self) -> Option<Self::Item> {

        #[cfg(feature = "circular_buffer")]
        if self.pos >= self.buffer.cap.min(self.buffer.filled_data_length)
        {
            return None;
        }
        #[cfg(not(feature = "circular_buffer"))]
        if self.pos >= self.buffer.filled_data_length {
            return None;
        }

        let item = unsafe { &mut *self.buffer.pointer.add(self.pos + self.buffer.start_pos) };
        self.pos += 1;
        Some(item)
    }
}
impl<T> Index<RangeFrom<usize>> for WaterBuffer<T> {
    type Output = [T];

    fn index(&self, idx: RangeFrom<usize>) -> &Self::Output {
        if idx.start > self.filled_data_length
        {
            panic!("Range out of bounds");
        }
        unsafe { std::slice::from_raw_parts(
            self.pointer.add(self.start_pos + idx.start ),
            self.filled_data_length)
        }
    }
}
impl<T> IndexMut<RangeFrom<usize>> for WaterBuffer<T> {
    fn index_mut(&mut self, idx: RangeFrom<usize>) -> &mut Self::Output {
        if idx.start > self.filled_data_length
        {
            panic!("Range out of bounds");
        }
        unsafe { std::slice::from_raw_parts_mut(
            self.pointer.add(self.start_pos + idx.start ),
            self.filled_data_length)
        }
    }
}impl<T> IndexMut<RangeTo<usize>> for WaterBuffer<T> {
    fn index_mut(&mut self, idx: RangeTo<usize>) -> &mut Self::Output {
        if idx.end > self.filled_data_length
        {
            panic!("Range out of bounds");
        }
        unsafe { std::slice::from_raw_parts_mut(
            self.pointer.add(self.start_pos  ),
            idx.end)
        }
    }
}
impl<T> Index<RangeTo<usize>> for WaterBuffer<T> {
    type Output = [T];

    fn index(&self, idx: RangeTo<usize>) -> &Self::Output {
        if idx.end > self.filled_data_length
        {
            panic!("Range out of bounds");
        }
        unsafe { std::slice::from_raw_parts(
            self.pointer.add(self.start_pos  ),
            idx.end)
        }
    }
}

/// Indexing implementations
impl<T> Index<Range<usize>> for WaterBuffer<T> {
    type Output = [T];

    fn index(&self, idx: Range<usize>) -> &Self::Output {
        if idx.start > idx.end || idx.end > self.filled_data_length
        {
            panic!("Range out of bounds");
        }
        unsafe { std::slice::from_raw_parts(self.pointer.add(self.start_pos + idx.start),
                                            self.start_pos + idx.end) }
    }
}

impl<T> IndexMut<Range<usize>> for WaterBuffer<T> {
    fn index_mut(&mut self, idx: Range<usize>) -> &mut Self::Output {
        if idx.start > idx.end
        {
            panic!("Range out of bounds");
        }
        unsafe { std::slice::from_raw_parts_mut(self.pointer.add(
            self.start_pos + idx.start), idx.end + self.start_pos) }
    }
}

impl<T> Index<RangeFull> for WaterBuffer<T> {
    type Output = [T];

    fn index(&self, _idx: RangeFull) -> &Self::Output {
        #[cfg(feature = "circular_buffer")]
        unsafe {
            if self.filled_data_length > self.cap {
                return std::slice::from_raw_parts(self.pointer,self.cap)
            }
        };
        unsafe { std::slice::from_raw_parts(self.pointer.add(self.start_pos), self.filled_data_length)}
    }
}

impl<T> IndexMut<RangeFull> for WaterBuffer<T> {
    fn index_mut(&mut self, _idx: RangeFull) -> &mut Self::Output {
        #[cfg(feature = "circular_buffer")]
        return unsafe {
            std::slice::from_raw_parts_mut(self.pointer.add(self.start_pos),
                                           if self.filled_data_length > self.cap {
                                               self.cap
                                           } else {
                                               self.filled_data_length
                                           })
        };
        #[cfg(not(feature = "circular_buffer"))]
        unsafe { std::slice::from_raw_parts_mut(self.pointer.add(self.start_pos), self.filled_data_length)}
    }
}

impl<T> Index<usize> for WaterBuffer<T> {
    type Output = T;

    fn index(&self,
             #[cfg(feature = "circular_buffer")]
             mut index: usize,
             #[cfg(not(feature = "circular_buffer"))]
             index:  usize,
    ) -> &Self::Output {

        #[cfg(feature = "circular_buffer")]
        {
            while index > self.filled_data_length
            {
                index -= self.filled_data_length
                ;
            }
        }

        #[cfg(not(feature = "circular_buffer"))]
        if index >= self.filled_data_length
        {
            panic!("Index out of bounds");
        }
        unsafe { &*self.pointer.add(index) }
    }
}

impl<T> IndexMut<usize> for WaterBuffer<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if index >= self.filled_data_length
        {
            panic!("Index out of bounds");
        }
        unsafe { &mut *self.pointer.add(index) }
    }
}

impl<T> Deref for WaterBuffer<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self[..]
    }
}
impl<T> DerefMut for WaterBuffer<T> {

    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self[..]
    }
}


#[cfg(feature = "uring")]
unsafe impl IoBuf for WaterBuffer<u8> {
    fn stable_ptr(&self) -> *const u8 {
        // We use the start_pos to ensure we point to the beginning of valid data
        unsafe { self.pointer.add(self.start_pos) }
    }

    fn bytes_init(&self) -> usize {
        self.filled_data_length
    }

    fn bytes_total(&self) -> usize {
        // The total capacity relative to the current start_pos
        self.capacity()
    }
}
#[cfg(feature = "uring")]

// IoBufMut for writing data INTO the buffer (reading from a socket)
unsafe impl IoBufMut for WaterBuffer<u8> {
    fn stable_mut_ptr(&mut self) -> *mut u8 {
        // Kernel writes starting after the already filled data
        unsafe { self.pointer.add(self.start_pos + self.filled_data_length) }
    }

    unsafe fn set_init(&mut self, pos: usize) {
        // After kernel writes 'pos' bytes, we update our internal counter
        self.filled_data_length += pos;
    }
}

