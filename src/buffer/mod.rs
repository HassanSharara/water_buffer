//! # WaterBuffer
//!
//! `WaterBuffer` is a generic dynamically-sized buffer in Rust, primarily for bytes (`u8`).  
//! It provides safe memory management, dynamic resizing, iteration, and indexing.  

use std::alloc::{alloc, dealloc, realloc, Layout};
use std::ops::{Index, IndexMut, Range, RangeFull};
use std::ptr;

type InnerType = u8;

/// Main dynamic buffer struct
pub struct WaterBuffer<T> {
    cap: usize,
    start_pos: usize,
    pointer: *mut T,
    filled_data_length: usize,
}

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

    /// Returns a mutable iterator over the buffer
    pub fn iter_mut(&mut self) -> WaterBufferIterMut<'_> {
        WaterBufferIterMut {
            buffer: self,
            pos: 0,
        }
    }

    /// Creates a new buffer with a given capacity
    pub fn with_capacity(cap: usize) -> WaterBuffer<InnerType> {
        let layout = Layout::array::<InnerType>(cap).unwrap();
        let first_element_pointer = unsafe { alloc(layout) } as *mut InnerType;
        WaterBuffer {
            cap,
            pointer: first_element_pointer,
            start_pos: 0,
            filled_data_length: 0,
        }
    }

    /// Expands the buffer to a new capacity
    #[inline(always)]
    pub fn expand(&mut self, n: usize) {
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

    /// Calculates an appropriate size for buffer growth
    #[inline(always)]
    const fn ap_size(&self, len: usize) -> usize {
        let re = self.cap + (self.cap / 2);
        if len > re {
            return len;
        }
        re
    }

    /// Extends the buffer from a slice
    #[inline(always)]
    pub fn extend_from_slice(&mut self, slice: &[u8]) {
        if self.filled_data_length + slice.len() > self.cap {
            self.expand(self.ap_size(self.filled_data_length + slice.len()));
        }
        unsafe {
            ptr::copy_nonoverlapping(
                slice.as_ptr(),
                self.pointer.add(self.filled_data_length) as *mut u8,
                slice.len(),
            )
        };
        self.filled_data_length += slice.len();
    }

    /// Returns the number of elements in the buffer
    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.filled_data_length
    }

    /// Resets the buffer
    #[inline(always)]
    pub const fn reset(&mut self) {
        self.filled_data_length = 0;
        self.start_pos = 0;
    }

    /// Pushes a single element into the buffer
    #[inline]
    pub fn push(&mut self, item: InnerType) {
        if self.filled_data_length >= self.cap {
            self.expand(self.ap_size(self.filled_data_length + 1));
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

    #[inline(always)]
    pub const fn clear(&mut self) {
        self.reset();
    }

    #[inline(always)]
    pub const fn advance(&mut self, n: usize) {
        if n > self.filled_data_length {
            panic!("Insufficient space to advance");
        }
        self.start_pos += n;
        self.filled_data_length -= n;
    }

    #[inline(always)]
    pub const fn remaining(&self) -> usize {
        self.filled_data_length - self.start_pos
    }

    #[inline(always)]
    pub const fn advance_mut(&mut self, n: usize) {
        self.start_pos += n;
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
        let current = self.iterator_pos;
        if current + 1 > self.buffer.filled_data_length {
            self.iterator_pos = 0;
            return None;
        }
        self.iterator_pos += 1;
        Some(self.buffer[current])
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
        if self.pos >= self.buffer.filled_data_length {
            return None;
        }
        let item = unsafe { &*self.buffer.pointer.add(self.pos) };
        self.pos += 1;
        Some(item)
    }
}

impl<'a> Iterator for WaterBufferIterMut<'a> {
    type Item = &'a mut u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.buffer.filled_data_length {
            return None;
        }
        let item = unsafe { &mut *self.buffer.pointer.add(self.pos) };
        self.pos += 1;
        Some(item)
    }
}

/// Indexing implementations
impl<T> Index<Range<usize>> for WaterBuffer<T> {
    type Output = [T];

    fn index(&self, idx: Range<usize>) -> &Self::Output {
        if idx.start > idx.end || idx.end > self.filled_data_length {
            panic!("Range out of bounds");
        }
        unsafe { std::slice::from_raw_parts(self.pointer.add(idx.start), idx.end - idx.start) }
    }
}

impl<T> IndexMut<Range<usize>> for WaterBuffer<T> {
    fn index_mut(&mut self, idx: Range<usize>) -> &mut Self::Output {
        if idx.start > idx.end || idx.end > self.filled_data_length {
            panic!("Range out of bounds");
        }
        unsafe { std::slice::from_raw_parts_mut(self.pointer.add(idx.start), idx.end - idx.start) }
    }
}

impl<T> Index<RangeFull> for WaterBuffer<T> {
    type Output = [T];

    fn index(&self, _idx: RangeFull) -> &Self::Output {
        unsafe { std::slice::from_raw_parts(self.pointer.add(self.start_pos), self.filled_data_length) }
    }
}

impl<T> IndexMut<RangeFull> for WaterBuffer<T> {
    fn index_mut(&mut self, _idx: RangeFull) -> &mut Self::Output {
        unsafe { std::slice::from_raw_parts_mut(self.pointer.add(self.start_pos), self.filled_data_length) }
    }
}

impl<T> Index<usize> for WaterBuffer<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        if index >= self.filled_data_length {
            panic!("Index out of bounds");
        }
        unsafe { &*self.pointer.add(index) }
    }
}

impl<T> IndexMut<usize> for WaterBuffer<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if index >= self.filled_data_length {
            panic!("Index out of bounds");
        }
        unsafe { &mut *self.pointer.add(index) }
    }
}
