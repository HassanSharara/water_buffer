//! # WaterBuffer - Vec-based Implementation
//!
//! `WaterBuffer` is a high-performance dynamically-sized buffer in Rust.
//! This version uses Vec internally for automatic memory management.

use std::mem::MaybeUninit;
use std::ops::{Deref, DerefMut, Index, IndexMut, Range, RangeFrom, RangeFull, RangeTo};
#[cfg(feature = "impl_bytes")]
use bytes::buf::UninitSlice;
#[cfg(feature = "impl_bytes")]
use bytes::BufMut;

type InnerType = u8;

#[derive(Debug)]
/// Main dynamic buffer struct with Vec-based memory management
pub struct WaterBuffer<T> {
    data: Vec<T>,
    pub(crate) start_pos: usize,
    #[cfg(feature = "circular_buffer")]
    pub(crate) circular_position: Option<usize>,
    pub(crate) filled_data_length: usize,
}

impl<T> WaterBuffer<T> {
    #[cfg(not(feature = "circular_buffer"))]
    #[inline(always)]
    pub const fn reset(&mut self) {
        self.start_pos = 0;
        self.filled_data_length = 0;
    }

    #[cfg(feature = "circular_buffer")]
    #[inline(always)]
    pub const fn reset(&mut self) {
        self.start_pos = 0;
        self.circular_position = None;
        self.filled_data_length = 0;
    }

    #[inline(always)]
    pub fn capacity(&self) -> usize {
        self.data.capacity() - self.start_pos
    }

    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.filled_data_length
    }

    #[inline(always)]
    pub fn cap(&self) -> usize {
        self.data.capacity()
    }
}

#[cfg(feature = "impl_bytes")]
unsafe impl BufMut for WaterBuffer<u8> {
    #[inline(always)]
    fn remaining_mut(&self) -> usize {
        self.len()
    }

    #[inline(always)]
    unsafe fn advance_mut(&mut self, cnt: usize) {
        self.advance_mut(cnt)
    }

    #[inline(always)]
    fn chunk_mut(&mut self) -> &mut UninitSlice {
        UninitSlice::uninit(self.chunk_mut_maybeunint())
    }
}

unsafe impl<T> Send for WaterBuffer<T> where T: Send {}
unsafe impl<T> Sync for WaterBuffer<T> where T: Sync {}

impl<T> Deref for WaterBuffer<T> {
    type Target = [T];

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self[..]
    }
}

impl<T> DerefMut for WaterBuffer<T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self[..]
    }
}

impl WaterBuffer<InnerType> {
    // --- Iterator Methods ---
    #[inline]
    pub fn into_owned_iter(self) -> WaterBufferOwnedIter<InnerType> {
        self.into()
    }

    #[inline]
    pub fn iter(&self) -> WaterBufferIter<'_> {
        WaterBufferIter {
            buffer: self,
            pos: 0,
        }
    }

    #[inline]
    pub fn iter_mut(&mut self) -> WaterBufferIterMut<'_> {
        WaterBufferIterMut {
            buffer: self,
            pos: 0,
        }
    }

    // --- Construction ---
    #[inline]
    pub fn with_capacity(cap: usize) -> WaterBuffer<InnerType> {
        #[cfg(feature = "circular_buffer")]
        {
            WaterBuffer {
                data: Vec::with_capacity(cap),
                start_pos: 0,
                filled_data_length: 0,
                circular_position: None,
            }
        }

        #[cfg(not(feature = "circular_buffer"))]
        WaterBuffer {
            data: Vec::with_capacity(cap),
            start_pos: 0,
            filled_data_length: 0,
        }
    }

    #[inline(always)]
    fn compact(&mut self) {
        if self.start_pos == 0 {
            return;
        }

        if self.filled_data_length == 0 {
            self.start_pos = 0;
            unsafe {
                self.data.set_len(0);
            }
            return;
        }

        // Shift data to the beginning
        self.data.copy_within(self.start_pos..self.start_pos + self.filled_data_length, 0);
        self.start_pos = 0;
        unsafe {
            self.data.set_len(self.filled_data_length);
        }
    }

    #[inline(always)]
    pub fn expand(&mut self, additional: usize) {
        let current_end = self.start_pos + self.filled_data_length;
        let required_capacity = current_end + additional;

        if self.data.capacity() < required_capacity {
            self.data.reserve(required_capacity - self.data.capacity());
        }
    }

    #[inline(always)]
    pub fn ensure_capacity(&mut self, additional: usize) {

        self.data.reserve(additional);
        return;
        let available = self.data.capacity() - (self.start_pos + self.filled_data_length);
        if additional <= available {
            return;
        }

        // Try compacting first if it would help
        if self.start_pos > 0 && (self.data.capacity() - self.filled_data_length) >= additional {
            self.compact();
        } else {
            self.expand(additional);
        }
    }

    // --- Push ---
    #[cfg(not(feature = "circular_buffer"))]
    #[inline(always)]
    pub fn push(&mut self, item: InnerType) {
        let current_end = self.start_pos + self.filled_data_length;

        if current_end >= self.data.capacity() {
            self.ensure_capacity(1);
        }

        unsafe {
            let ptr = self.data.as_mut_ptr().add(self.start_pos + self.filled_data_length);
            std::ptr::write(ptr, item);
        }
        self.filled_data_length += 1;

        // Update Vec's length to match
        unsafe {
            self.data.set_len((self.start_pos + self.filled_data_length).max(self.data.len()));
        }
    }

    #[cfg(feature = "circular_buffer")]
    #[inline(always)]
    pub fn push(&mut self, item: InnerType) {
        let cap = self.data.capacity();
        if self.filled_data_length >= cap {
            let p = self.circular_position.as_ref().unwrap_or(&0);
            unsafe {
                *self.data.as_mut_ptr().add(*p) = item;
            }
            let n = *p + 1;
            self.circular_position = Some(if n >= cap { 0 } else { n });
            return;
        }
        unsafe {
            let ptr = self.data.as_mut_ptr().add(self.filled_data_length);
            std::ptr::write(ptr, item);
        }
        self.filled_data_length += 1;
        unsafe {
            self.data.set_len(self.filled_data_length.max(self.data.len()));
        }
    }

    #[cfg(feature = "circular_buffer")]
    #[inline(always)]
    pub fn extend_from_slice(&mut self, mut slice: &[u8]) {
        let cap = self.data.capacity();
        if cap == 0 {
            return;
        }
        let mut must_write_len = slice.len();

        while must_write_len > 0 {
            let mut position_to_write = self.circular_position.unwrap_or_else(|| {
                let filled = self.filled_data_length + self.start_pos;
                if filled >= cap {
                    self.circular_position = Some(0);
                    return 0;
                }
                filled
            });

            if position_to_write >= cap {
                position_to_write = 0;
            }

            let available_len = (cap - position_to_write).min(must_write_len);
            let n_slice = &slice[..available_len];

            unsafe {
                std::ptr::copy_nonoverlapping(
                    n_slice.as_ptr(),
                    self.data.as_mut_ptr().add(position_to_write),
                    n_slice.len()
                );
            }

            if self.filled_data_length < cap {
                self.filled_data_length += available_len;
                unsafe {
                    self.data.set_len(self.filled_data_length.max(self.data.len()));
                }
            } else {
                if let Some(cp) = self.circular_position.as_mut() {
                    *cp += available_len;
                    if *cp >= cap {
                        *cp = 0;
                    }
                } else {
                    let mut p = position_to_write + available_len;
                    if p >= cap {
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
    #[inline(always)]
    pub fn extend_from_slice(&mut self, slice: &[u8]) {
        let additional = slice.len();
        if additional == 0 {
            return;
        }

        self.ensure_capacity(additional);

        unsafe {
            let write_ptr = self.data.as_mut_ptr().add(self.start_pos + self.filled_data_length);
            std::ptr::copy_nonoverlapping(slice.as_ptr(), write_ptr, additional);
        }
        self.filled_data_length += additional;

        unsafe {
            self.data.set_len((self.start_pos + self.filled_data_length).max(self.data.len()));
        }
    }

    #[inline(always)]
    pub const fn clear(&mut self) {
        self.start_pos = 0;
        self.filled_data_length = 0;
    }

    #[inline(always)]
    pub fn advance(&mut self, n: usize) {
        if n > self.filled_data_length {
            panic!("Insufficient space to advance");
        }
        self.start_pos += n;
        self.filled_data_length -= n;
        if self.filled_data_length == 0 {
            self.reset();
            unsafe {
                self.data.set_len(0);
            }
        }
    }

    #[inline(always)]
    pub const fn remaining(&self) -> usize {
        self.len()
    }

    #[cfg(not(feature = "circular_buffer"))]
    #[inline(always)]
    pub fn un_initialized_remaining(&self) -> usize {
        let cap = self.data.capacity();
        let available = cap - self.start_pos;
        if self.filled_data_length >= available {
            return 0;
        }
        available - self.filled_data_length
    }

    #[cfg(not(feature = "circular_buffer"))]
    #[inline(always)]
    pub fn chunk_mut_maybeunint<T>(&mut self) -> &mut [MaybeUninit<T>] {
        unsafe {
            let pos = self.start_pos + self.filled_data_length;
            let cap = self.data.capacity();
            let pointer = self.data.as_mut_ptr().add(pos) as *mut MaybeUninit<T>;
            std::slice::from_raw_parts_mut(pointer, cap - pos)
        }
    }

    #[cfg(not(feature = "circular_buffer"))]
    #[inline(always)]
    pub fn chunk_mut(&mut self) -> &mut [u8] {
        unsafe {
            let pos = self.start_pos + self.filled_data_length;
            let cap = self.data.capacity();
            let pointer = self.data.as_mut_ptr().add(pos);
            std::slice::from_raw_parts_mut(pointer, cap - pos)
        }
    }

    #[inline(always)]
    pub fn chunk(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                self.data.as_ptr().add(self.start_pos),
                self.filled_data_length
            )
        }
    }

    #[inline(always)]
    pub const fn advance_mut(&mut self, n: usize) {
        self.filled_data_length += n;
    }
}

// --- Trait Implementations ---

impl Into<WaterBufferOwnedIter<InnerType>> for WaterBuffer<InnerType> {
    fn into(self) -> WaterBufferOwnedIter<InnerType> {
        WaterBufferOwnedIter {
            buffer: self,
            iterator_pos: 0,
        }
    }
}

// --- Iterator Structs ---

pub struct WaterBufferOwnedIter<InnerType> {
    iterator_pos: usize,
    buffer: WaterBuffer<InnerType>,
}

impl Iterator for WaterBufferOwnedIter<InnerType> {
    type Item = InnerType;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        #[cfg(feature = "circular_buffer")]
        if self.iterator_pos >= self.buffer.cap().min(self.buffer.filled_data_length) {
            return None;
        }
        #[cfg(not(feature = "circular_buffer"))]
        if self.iterator_pos >= self.buffer.filled_data_length {
            return None;
        }

        let item = unsafe { *self.buffer.data.as_ptr().add(self.iterator_pos + self.buffer.start_pos) };
        self.iterator_pos += 1;
        Some(item)
    }
}

pub struct WaterBufferIter<'a> {
    buffer: &'a WaterBuffer<u8>,
    pos: usize,
}

pub struct WaterBufferIterMut<'a> {
    buffer: &'a mut WaterBuffer<u8>,
    pos: usize,
}

impl<'a> Iterator for WaterBufferIter<'a> {
    type Item = &'a u8;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        #[cfg(feature = "circular_buffer")]
        if self.pos >= self.buffer.cap().min(self.buffer.filled_data_length) {
            return None;
        }
        #[cfg(not(feature = "circular_buffer"))]
        if self.pos >= self.buffer.filled_data_length {
            return None;
        }

        let item = unsafe { &*self.buffer.data.as_ptr().add(self.pos + self.buffer.start_pos) };
        self.pos += 1;
        Some(item)
    }
}

impl<'a> Iterator for WaterBufferIterMut<'a> {
    type Item = &'a mut u8;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        #[cfg(feature = "circular_buffer")]
        if self.pos >= self.buffer.cap().min(self.buffer.filled_data_length) {
            return None;
        }
        #[cfg(not(feature = "circular_buffer"))]
        if self.pos >= self.buffer.filled_data_length {
            return None;
        }

        let item = unsafe { &mut *self.buffer.data.as_mut_ptr().add(self.pos + self.buffer.start_pos) };
        self.pos += 1;
        Some(item)
    }
}

// --- Index Implementations ---

impl<T> Index<Range<usize>> for WaterBuffer<T> {
    type Output = [T];

    #[inline]
    fn index(&self, idx: Range<usize>) -> &Self::Output {
        if idx.start > idx.end || idx.end > self.filled_data_length {
            panic!("Range out of bounds");
        }
        unsafe {
            std::slice::from_raw_parts(
                self.data.as_ptr().add(self.start_pos + idx.start),
                idx.end - idx.start
            )
        }
    }
}

impl<T> Index<RangeFrom<usize>> for WaterBuffer<T> {
    type Output = [T];

    #[inline]
    fn index(&self, idx: RangeFrom<usize>) -> &Self::Output {
        if idx.start > self.filled_data_length {
            panic!("Range out of bounds");
        }
        unsafe {
            std::slice::from_raw_parts(
                self.data.as_ptr().add(self.start_pos + idx.start),
                self.filled_data_length - idx.start
            )
        }
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
            std::slice::from_raw_parts(self.data.as_ptr().add(self.start_pos), index.end)
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
            std::slice::from_raw_parts_mut(self.data.as_mut_ptr().add(self.start_pos), index.end)
        }
    }
}

impl<T> IndexMut<RangeFrom<usize>> for WaterBuffer<T> {
    #[inline]
    fn index_mut(&mut self, idx: RangeFrom<usize>) -> &mut Self::Output {
        if idx.start > self.filled_data_length {
            panic!("Range out of bounds");
        }
        unsafe {
            std::slice::from_raw_parts_mut(
                self.data.as_mut_ptr().add(self.start_pos + idx.start),
                self.filled_data_length - idx.start
            )
        }
    }
}

impl<T> IndexMut<Range<usize>> for WaterBuffer<T> {
    #[inline]
    fn index_mut(&mut self, idx: Range<usize>) -> &mut Self::Output {
        if idx.start > idx.end || idx.end > self.filled_data_length {
            panic!("Range out of bounds");
        }
        unsafe {
            std::slice::from_raw_parts_mut(
                self.data.as_mut_ptr().add(self.start_pos + idx.start),
                idx.end - idx.start
            )
        }
    }
}

impl<T> Index<RangeFull> for WaterBuffer<T> {
    type Output = [T];

    #[inline]
    fn index(&self, _idx: RangeFull) -> &Self::Output {
        #[cfg(feature = "circular_buffer")]
        unsafe {
            let cap = self.data.capacity();
            if self.filled_data_length > cap {
                return std::slice::from_raw_parts(self.data.as_ptr(), cap);
            }
        }
        unsafe {
            std::slice::from_raw_parts(
                self.data.as_ptr().add(self.start_pos),
                self.filled_data_length
            )
        }
    }
}

impl<T> IndexMut<RangeFull> for WaterBuffer<T> {
    #[inline]
    fn index_mut(&mut self, _idx: RangeFull) -> &mut Self::Output {
        #[cfg(feature = "circular_buffer")]
        {
            let cap = self.data.capacity();
            return unsafe {
                std::slice::from_raw_parts_mut(
                    self.data.as_mut_ptr().add(self.start_pos),
                    if self.filled_data_length > cap {
                        cap
                    } else {
                        self.filled_data_length
                    }
                )
            };
        }
        #[cfg(not(feature = "circular_buffer"))]
        unsafe {
            std::slice::from_raw_parts_mut(
                self.data.as_mut_ptr().add(self.start_pos),
                self.filled_data_length
            )
        }
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
        unsafe { &*self.data.as_ptr().add(self.start_pos + index) }
    }
}

impl<T> IndexMut<usize> for WaterBuffer<T> {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if index >= self.filled_data_length {
            panic!("Index out of bounds");
        }
        unsafe { &mut *self.data.as_mut_ptr().add(self.start_pos + index) }
    }
}