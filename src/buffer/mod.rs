//! # WaterBuffer
//!
//! `WaterBuffer` is a generic dynamically-sized buffer in Rust, primarily for bytes (`u8`).  
//! It provides safe memory management, dynamic resizing, iteration, and indexing.  

use std::alloc::{alloc, dealloc, realloc, Layout};
use std::mem::MaybeUninit;
use std::ops::{Deref, DerefMut, Index, IndexMut, Range, RangeFrom, RangeFull, RangeTo};
use std::ptr;
#[cfg(feature = "impl_bytes")]
use bytes::buf::UninitSlice;
#[cfg(feature = "impl_bytes")]
use bytes::BufMut;

type InnerType = u8;

#[derive(Debug)]
/// Main dynamic buffer struct
pub struct WaterBuffer<T> {
    pub (crate) cap: usize,
    pub(crate) start_pos: usize,
    #[cfg(feature = "circular_buffer")]
    pub (crate) circular_position: Option<usize>,
    pub (crate) pointer: *mut T,
    pub (crate) filled_data_length: usize,
}

#[cfg(feature = "impl_bytes")]
unsafe impl BufMut for WaterBuffer<u8> {
    fn remaining_mut(&self) -> usize {
        self.len()
    }

    unsafe fn advance_mut(&mut self, cnt: usize) {
        self.advance_mut(cnt)
    }

    fn chunk_mut(&mut self) -> &mut UninitSlice {
        UninitSlice::uninit(self.chunk_mut_maybeunint())
    }
}

unsafe impl<T> Send for WaterBuffer<T> {}

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

impl WaterBuffer<InnerType> {
    /// Converts the buffer into an owned iterator
    #[inline]
    pub fn into_owned_iter(self) -> WaterBufferOwnedIter<InnerType> {
        self.into()
    }

    /// Returns an immutable iterator over the buffer
    #[inline]
    pub fn iter(&self) -> WaterBufferIter<'_> {
        WaterBufferIter {
            buffer: self,
            pos: 0,
        }
    }

    /// Returns a mutable iterator over the buffer
    #[inline]
    pub fn iter_mut(&mut self) -> WaterBufferIterMut<'_> {
        WaterBufferIterMut {
            buffer: self,
            pos: 0,
        }
    }

    /// Creates a new buffer with a given capacity
    #[inline]
    pub fn with_capacity(cap: usize) -> WaterBuffer<InnerType> {
        if cap == 0 {
            #[cfg(feature = "circular_buffer")]
            {
                return WaterBuffer {
                    cap: 0,
                    pointer: std::ptr::null_mut(),
                    start_pos: 0,
                    filled_data_length: 0,
                    circular_position: None,
                };
            }
            #[cfg(not(feature = "circular_buffer"))]
            {
                return WaterBuffer {
                    cap: 0,
                    pointer: std::ptr::null_mut(),
                    start_pos: 0,
                    filled_data_length: 0,
                };
            }
        }

        let layout = Layout::array::<InnerType>(cap).unwrap();
        let first_element_pointer = unsafe { alloc(layout) } as *mut InnerType;

        #[cfg(feature = "circular_buffer")]
        {
            WaterBuffer {
                cap,
                pointer: first_element_pointer,
                start_pos: 0,
                filled_data_length: 0,
                circular_position: None,
            }
        }

        #[cfg(not(feature = "circular_buffer"))]
        WaterBuffer {
            cap,
            pointer: first_element_pointer,
            start_pos: 0,
            filled_data_length: 0,
        }
    }

    /// ULTRA-OPTIMIZED: Aggressive growth for maximum performance
    #[inline]
    const fn calculate_new_capacity(&self, required: usize) -> usize {
        if self.cap == 0 {
            return  required;
        }

        // AGGRESSIVE STRATEGY: Always double until 4MB
        // This minimizes reallocations at the cost of some memory
        if self.cap < 4 * 1024 * 1024 {
            let doubled = self.cap * 2;
            if required > doubled {
                // For huge jumps, round to next power of 2 + 25%
                let mut n = required;
                n = n - 1;
                n = n | (n >> 1);
                n = n | (n >> 2);
                n = n | (n >> 4);
                n = n | (n >> 8);
                n = n | (n >> 16);
                n = n | (n >> 32);
                let next_pow2 = n + 1;
                // Add 25% buffer for future growth
                next_pow2 + (next_pow2 >> 2)
            } else {
                doubled
            }
        } else {
            // Only for very large buffers (>4MB), use 1.5x
            let growth = self.cap + (self.cap >> 1);
            if required > growth {
                required + (required >> 1) // Still add 50% padding
            } else {
                growth
            }
        }
    }

    /// Expands the buffer to a new capacity
    #[inline]
    pub fn expand(&mut self, n: usize) {
        debug_assert!(n >= self.cap, "New capacity must be >= current capacity");

        if self.cap == 0 {
            let layout = Layout::array::<InnerType>(n).unwrap();
            self.pointer = unsafe { alloc(layout) } as *mut InnerType;
            self.cap = n;
            return;
        }

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
    /// OPTIMIZED: Better growth strategy
    #[inline]
    pub(crate) const fn ap_size(&self, len: usize) -> usize {
        self.calculate_new_capacity(len)
    }

    #[cfg(feature = "circular_buffer")]
    /// Extends the buffer from a slice
    #[inline(always)]
    pub fn extend_from_slice(&mut self, mut slice: &[u8]) {
        if self.cap == 0 {
            return;
        }
        let mut must_write_len = slice.len();

        while must_write_len > 0 {
            let mut position_to_write = self.circular_position.unwrap_or_else(|| {
                let filled = self.filled_data_length + self.start_pos;
                if filled >= self.cap {
                    self.circular_position = Some(0);
                    return 0;
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
            }

            if self.filled_data_length < self.cap {
                self.filled_data_length += available_len;
            } else {
                if let Some(cp) = self.circular_position.as_mut() {
                    *cp += available_len;
                    if *cp >= self.cap {
                        *cp = 0;
                    }
                } else {
                    let mut p = position_to_write + available_len;
                    if p >= self.cap {
                        p = 0;
                    }
                    self.circular_position = Some(p);
                }
            }
            must_write_len -= available_len;
            slice = &slice[available_len..];
        }
    }

    #[cfg(not(feature = "circular_buffer"))]
    /// OPTIMIZED: Hot/cold path separation for extend_from_slice
    #[inline(always)]
    pub fn extend_from_slice(&mut self, slice: &[u8]) {
        if slice.is_empty() {
            return;
        }

        let required = self.filled_data_length + slice.len();

        if required > self.cap {
            self.extend_from_slice_cold(slice, required);
            return;
        }

        // Hot path: no reallocation - use faster method for small/large writes
        if slice.len() <= 8 {
            // For tiny slices, unrolled copy is faster
            unsafe {
                let dst = self.pointer.add(self.start_pos + self.filled_data_length);
                let src = slice.as_ptr();
                for i in 0..slice.len() {
                    *dst.add(i) = *src.add(i);
                }
            }
        } else {
            // For larger slices, use copy_nonoverlapping
            unsafe {
                ptr::copy_nonoverlapping(
                    slice.as_ptr(),
                    self.pointer.add(self.start_pos + self.filled_data_length),
                    slice.len(),
                );
            }
        }
        self.filled_data_length += slice.len();
    }

    #[cfg(not(feature = "circular_buffer"))]
    /// Cold path for extend with reallocation - ULTRA OPTIMIZED
    #[inline(never)]
    #[cold]
    fn extend_from_slice_cold(&mut self, slice: &[u8], required: usize) {
        // Calculate new capacity with extra buffer for future writes
        let new_cap = self.calculate_new_capacity(required);
        self.expand(new_cap);

        unsafe {
            ptr::copy_nonoverlapping(
                slice.as_ptr(),
                self.pointer.add(self.start_pos + self.filled_data_length),
                slice.len(),
            );
        }
        self.filled_data_length += slice.len();
    }

    /// Returns the number of elements in the buffer
    #[inline(always)]
    pub const fn len(&self) -> usize {
        #[cfg(feature = "circular_buffer")]
        {
            if self.filled_data_length >= self.cap {
                return self.cap;
            }
        }
        self.filled_data_length
    }

    #[cfg(feature = "circular_buffer")]
    #[inline(always)]
    pub const fn reset(&mut self) {
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

    #[cfg(feature = "circular_buffer")]
    /// OPTIMIZED: Direct write instead of copy_nonoverlapping for push
    #[inline(always)]
    pub fn push(&mut self, item: InnerType) {
        if self.filled_data_length >= self.cap {
            let p = self.circular_position.as_ref().unwrap_or(&0);
            unsafe {
                *self.pointer.add(*p) = item;
            }
            let n = *p + 1;
            self.circular_position = Some(if n >= self.cap { 0 } else { n });
            return;
        }
        unsafe {
            *self.pointer.add(self.filled_data_length) = item;
        }
        self.filled_data_length += 1;
    }

    #[cfg(not(feature = "circular_buffer"))]
    /// OPTIMIZED: Hot/cold path separation + direct write
    #[inline(always)]
    pub fn push(&mut self, item: InnerType) {
        if self.filled_data_length >= self.cap {
            self.push_cold(item);
            return;
        }

        // Hot path: direct write (faster than copy_nonoverlapping)
        unsafe {
            *self.pointer.add(self.filled_data_length) = item;
        }
        self.filled_data_length += 1;
    }

    #[cfg(not(feature = "circular_buffer"))]
    /// Cold path for push with reallocation
    #[inline(never)]
    #[cold]
    fn push_cold(&mut self, item: InnerType) {
        let new_cap = if self.cap == 0 {
            64  // Larger initial size
        } else {
            self.cap * 2
        };
        self.expand(new_cap);

        unsafe {
            *self.pointer.add(self.filled_data_length) = item;
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
        self.len()
    }

    #[cfg(feature = "circular_buffer")]
    #[inline(always)]
    pub const fn un_initialized_remaining(&self) -> usize {
        let mut pos = self.filled_data_length;
        if let Some(p) = self.circular_position {
            if p > 0 {
                pos = p;
            }
        }
        if pos > self.cap {
            return 0;
        }
        self.cap - pos
    }

    #[cfg(not(feature = "circular_buffer"))]
    #[inline(always)]
    pub const fn un_initialized_remaining(&self) -> usize {
        self.cap - self.filled_data_length
    }

    #[cfg(not(feature = "circular_buffer"))]
    #[inline(always)]
    pub const fn chunk_mut_maybeunint<T>(&mut self) -> &mut [MaybeUninit<T>] {
        unsafe {
            let pos = self.start_pos + self.filled_data_length;
            let pointer = self.pointer.add(pos) as *mut MaybeUninit<T>;
            std::slice::from_raw_parts_mut(pointer, self.cap - pos)
        }
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

    #[cfg(all(feature = "circular_buffer"))]
    #[inline(always)]
    pub const fn chunk_mut_maybeunint<T>(&mut self) -> &mut [MaybeUninit<T>] {
        unsafe {
            let pos = self.start_pos + self.filled_data_length;
            if pos >= self.cap {
                return &mut [];
            }
            let pointer = self.pointer.add(pos) as *mut MaybeUninit<T>;
            std::slice::from_raw_parts_mut(pointer, self.cap - pos)
        }
    }

    #[inline(always)]
    pub const fn chunk(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(self.pointer, self.filled_data_length)
        }
    }

    #[cfg(feature = "circular_buffer")]
    #[inline(always)]
    pub const fn chunk_mut(&mut self) -> &mut [u8] {
        unsafe {
            let pos = self.start_pos + self.filled_data_length;
            if pos >= self.cap {
                return &mut [];
            }
            let pointer = self.pointer.add(pos);
            std::slice::from_raw_parts_mut(pointer, self.cap - pos)
        }
    }

    #[inline(always)]
    pub const fn advance_mut(&mut self, n: usize) {
        self.filled_data_length += n;
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

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.iterator_pos >= self.buffer.cap.min(self.buffer.filled_data_length) {
            return None;
        }
        #[cfg(not(feature = "circular_buffer"))]
        if self.iterator_pos >= self.buffer.filled_data_length {
            return None;
        }

        let item = unsafe { *self.buffer.pointer.add(self.iterator_pos + self.buffer.start_pos) };
        self.iterator_pos += 1;
        Some(item)
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

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        #[cfg(feature = "circular_buffer")]
        if self.pos >= self.buffer.cap.min(self.buffer.filled_data_length) {
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

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        #[cfg(feature = "circular_buffer")]
        if self.pos >= self.buffer.cap.min(self.buffer.filled_data_length) {
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

/// Indexing implementations
impl<T> Index<Range<usize>> for WaterBuffer<T> {
    type Output = [T];

    #[inline]
    fn index(&self, idx: Range<usize>) -> &Self::Output {
        if idx.start > idx.end || idx.end > self.filled_data_length {
            panic!("Range out of bounds");
        }
        unsafe { std::slice::from_raw_parts(self.pointer.add(idx.start), idx.end - idx.start) }
    }
}

impl<T> Index<RangeFrom<usize>> for WaterBuffer<T> {
    type Output = [T];

    #[inline]
    fn index(&self, idx: RangeFrom<usize>) -> &Self::Output {
        if idx.start > self.filled_data_length {
            panic!("Range out of bounds");
        }
        unsafe { std::slice::from_raw_parts(self.pointer.add(idx.start), self.filled_data_length - idx.start) }
    }
}

impl<T> Index<RangeTo<usize>> for WaterBuffer<T> {
    type Output = [T];

    #[inline]
    fn index(&self, index: RangeTo<usize>) -> &Self::Output {
        if index.end > self.filled_data_length {
            panic!("Range out of bounds");
        }
        unsafe {
            std::slice::from_raw_parts(self.pointer, index.end)
        }
    }
}

impl<T> IndexMut<RangeTo<usize>> for WaterBuffer<T> {
    #[inline]
    fn index_mut(&mut self, index: RangeTo<usize>) -> &mut Self::Output {
        if index.end > self.filled_data_length {
            panic!("Range out of bounds");
        }
        unsafe {
            std::slice::from_raw_parts_mut(self.pointer, index.end)
        }
    }
}

impl<T> IndexMut<RangeFrom<usize>> for WaterBuffer<T> {
    #[inline]
    fn index_mut(&mut self, idx: RangeFrom<usize>) -> &mut Self::Output {
        if idx.start > self.filled_data_length {
            panic!("Range out of bounds");
        }
        unsafe { std::slice::from_raw_parts_mut(self.pointer.add(idx.start), self.filled_data_length - idx.start) }
    }
}

impl<T> IndexMut<Range<usize>> for WaterBuffer<T> {
    #[inline]
    fn index_mut(&mut self, idx: Range<usize>) -> &mut Self::Output {
        if idx.start > idx.end {
            panic!("Range out of bounds");
        }
        unsafe { std::slice::from_raw_parts_mut(self.pointer.add(idx.start), idx.end - idx.start) }
    }
}

impl<T> Index<RangeFull> for WaterBuffer<T> {
    type Output = [T];

    #[inline]
    fn index(&self, _idx: RangeFull) -> &Self::Output {
        #[cfg(feature = "circular_buffer")]
        unsafe {
            if self.filled_data_length > self.cap {
                return std::slice::from_raw_parts(self.pointer, self.cap);
            }
        }
        unsafe { std::slice::from_raw_parts(self.pointer.add(self.start_pos), self.filled_data_length) }
    }
}

impl<T> IndexMut<RangeFull> for WaterBuffer<T> {
    #[inline]
    fn index_mut(&mut self, _idx: RangeFull) -> &mut Self::Output {
        #[cfg(feature = "circular_buffer")]
        return unsafe {
            std::slice::from_raw_parts_mut(
                self.pointer.add(self.start_pos),
                if self.filled_data_length > self.cap {
                    self.cap
                } else {
                    self.filled_data_length
                }
            )
        };
        #[cfg(not(feature = "circular_buffer"))]
        unsafe { std::slice::from_raw_parts_mut(self.pointer.add(self.start_pos), self.filled_data_length) }
    }
}

impl<T> Index<usize> for WaterBuffer<T> {
    type Output = T;

    #[inline]
    fn index(
        &self,
        #[cfg(feature = "circular_buffer")] mut index: usize,
        #[cfg(not(feature = "circular_buffer"))] index: usize,
    ) -> &Self::Output {
        #[cfg(feature = "circular_buffer")]
        {
            while index > self.filled_data_length {
                index -= self.filled_data_length;
            }
        }

        #[cfg(not(feature = "circular_buffer"))]
        if index >= self.filled_data_length {
            panic!("Index out of bounds");
        }
        unsafe { &*self.pointer.add(index) }
    }
}

impl<T> IndexMut<usize> for WaterBuffer<T> {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if index >= self.filled_data_length {
            panic!("Index out of bounds");
        }
        unsafe { &mut *self.pointer.add(index) }
    }
}