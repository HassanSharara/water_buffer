//! # WaterBuffer - Ultra High-Performance Vec-based Implementation
//!
//! `WaterBuffer` is a high-performance dynamically-sized buffer in Rust.
//! This version uses Vec internally with aggressive performance optimizations.

use std::mem::MaybeUninit;
use std::ops::{Deref, DerefMut, Index, IndexMut, Range, RangeFrom, RangeFull, RangeTo};
use std::ptr;
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

// Branch prediction hints
#[inline(always)]
#[cold]
const fn cold() {}

#[inline(always)]
fn likely(b: bool) -> bool {
    if !b {
        cold();
    }
    b
}

#[inline(always)]
fn unlikely(b: bool) -> bool {
    if b {
        cold();
    }
    b
}

impl<T> WaterBuffer<T> {
    #[cfg(not(feature = "circular_buffer"))]
    #[inline(always)]
    pub fn reset(&mut self) {
        self.start_pos = 0;
        self.filled_data_length = 0;
    }

    #[cfg(feature = "circular_buffer")]
    #[inline(always)]
    pub fn reset(&mut self) {
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

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.filled_data_length == 0
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
        let mut data = Vec::with_capacity(cap);
        // CRITICAL: Pre-set length to capacity to avoid reallocation issues
        // This ensures the entire capacity is usable immediately
        unsafe {
            data.set_len(cap);
        }

        #[cfg(feature = "circular_buffer")]
        {
            WaterBuffer {
                data,
                start_pos: 0,
                filled_data_length: 0,
                circular_position: None,
            }
        }

        #[cfg(not(feature = "circular_buffer"))]
        WaterBuffer {
            data,
            start_pos: 0,
            filled_data_length: 0,
        }
    }

    #[inline]
    pub fn new() -> WaterBuffer<InnerType> {
        #[cfg(feature = "circular_buffer")]
        {
            WaterBuffer {
                data: Vec::new(),
                start_pos: 0,
                filled_data_length: 0,
                circular_position: None,
            }
        }

        #[cfg(not(feature = "circular_buffer"))]
        WaterBuffer {
            data: Vec::new(),
            start_pos: 0,
            filled_data_length: 0,
        }
    }

    // --- Compaction (zero-cost when possible) ---
    #[inline(always)]
    fn compact(&mut self) {
        if self.start_pos == 0 {
            return;
        }

        if self.filled_data_length == 0 {
            self.start_pos = 0;
            return;
        }

        // Use ptr::copy for maximum performance (allows overlapping regions)
        unsafe {
            ptr::copy(
                self.data.as_ptr().add(self.start_pos),
                self.data.as_mut_ptr(),
                self.filled_data_length,
            );
        }
        self.start_pos = 0;
    }

    // --- Expansion with aggressive growth ---
    #[inline(always)]
    pub fn expand(&mut self, additional: usize) {
        let current_cap = self.data.capacity();
        let current_end = self.start_pos + self.filled_data_length;
        let required = current_end + additional;

        if required <= current_cap {
            return;
        }

        // Compact first if beneficial
        if self.start_pos > 0 {
            let available_after_compact = current_cap - self.filled_data_length;
            if available_after_compact >= additional {
                self.compact();
                return;
            }
        }

        // Calculate new capacity - always at least double
        let new_cap = required.max(current_cap * 2).max(128);

        // Reserve the difference
        let to_reserve = new_cap.saturating_sub(current_cap);
        self.data.reserve(to_reserve);

        // Update Vec length to match capacity
        unsafe {
            self.data.set_len(self.data.capacity());
        }
    }

    // --- Optimized reserve ---
    #[inline(always)]
    pub fn reserve(&mut self, additional: usize) {
        let current_end = self.start_pos + self.filled_data_length;
        let available = self.data.capacity().saturating_sub(current_end);

        if likely(additional <= available) {
            return;
        }

        self.expand(additional);
    }

    #[inline(always)]
    pub fn ensure_capacity(&mut self, additional: usize) {
        self.reserve(additional);
    }

    // --- Push (maximum performance hot path) ---
    #[cfg(not(feature = "circular_buffer"))]
    #[inline(always)]
    pub fn push(&mut self, item: InnerType) {
        let current_end = self.start_pos + self.filled_data_length;

        if unlikely(current_end >= self.data.capacity()) {
            self.expand(1);
        }

        unsafe {
            *self.data.as_mut_ptr().add(current_end) = item;
        }
        self.filled_data_length += 1;
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
            *self.data.as_mut_ptr().add(self.filled_data_length) = item;
        }
        self.filled_data_length += 1;
    }

    // --- Extend from slice (optimized bulk copy) ---
    #[cfg(not(feature = "circular_buffer"))]
    #[inline(always)]
    pub fn extend_from_slice(&mut self, slice: &[u8]) {
        let cnt = slice.len();
        if cnt == 0 {
            return;
        }

        let current_end = self.start_pos + self.filled_data_length;
        let available = self.data.capacity().saturating_sub(current_end);

        if unlikely(cnt > available) {
            self.expand(cnt);
        }

        unsafe {
            ptr::copy_nonoverlapping(
                slice.as_ptr(),
                self.data.as_mut_ptr().add(self.start_pos + self.filled_data_length),
                cnt,
            );
        }
        self.filled_data_length += cnt;
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
                ptr::copy_nonoverlapping(
                    n_slice.as_ptr(),
                    self.data.as_mut_ptr().add(position_to_write),
                    n_slice.len()
                );
            }

            if self.filled_data_length < cap {
                self.filled_data_length += available_len;
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

    // --- Other Methods ---
    #[inline(always)]
    pub fn clear(&mut self) {
        self.start_pos = 0;
        self.filled_data_length = 0;
    }

    #[inline(always)]
    pub fn advance(&mut self, n: usize) {
        if n > self.filled_data_length {
            panic!("Insufficient space to advance");
        }

        if n == 0 {
            return;
        }

        self.start_pos += n;
        self.filled_data_length -= n;

        if self.filled_data_length == 0 {
            self.reset();
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
        let used = self.start_pos + self.filled_data_length;
        cap.saturating_sub(used)
    }

    #[cfg(not(feature = "circular_buffer"))]
    #[inline(always)]
    pub fn chunk_mut_maybeunint<T>(&mut self) -> &mut [MaybeUninit<T>] {
        unsafe {
            let pos = self.start_pos + self.filled_data_length;
            let cap = self.data.capacity();
            let pointer = self.data.as_mut_ptr().add(pos) as *mut MaybeUninit<T>;
            std::slice::from_raw_parts_mut(pointer, cap.saturating_sub(pos))
        }
    }

    #[cfg(not(feature = "circular_buffer"))]
    #[inline(always)]
    pub fn chunk_mut(&mut self) -> &mut [u8] {
        unsafe {
            let pos = self.start_pos + self.filled_data_length;
            let cap = self.data.capacity();
            let pointer = self.data.as_mut_ptr().add(pos);
            std::slice::from_raw_parts_mut(pointer, cap.saturating_sub(pos))
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
    pub fn advance_mut(&mut self, n: usize) {
        self.filled_data_length += n;
    }

    /// Splits the buffer at the given index, returning the tail
    #[inline]
    pub fn split_off(&mut self, at: usize) -> WaterBuffer<InnerType> {
        assert!(
            at <= self.capacity(),
            "split_off out of bounds: {:?} <= {:?}",
            at,
            self.capacity(),
        );

        let other_start = self.start_pos + at;
        let other_len = self.filled_data_length.saturating_sub(at);

        // Create new buffer from the split data
        let mut other_data = Vec::with_capacity(other_len);
        if other_len > 0 {
            unsafe {
                other_data.set_len(other_len);
                ptr::copy_nonoverlapping(
                    self.data.as_ptr().add(other_start),
                    other_data.as_mut_ptr(),
                    other_len
                );
            }
        }

        // Update self
        self.filled_data_length = at.min(self.filled_data_length);

        #[cfg(feature = "circular_buffer")]
        {
            WaterBuffer {
                data: other_data,
                start_pos: 0,
                filled_data_length: other_len,
                circular_position: None,
            }
        }

        #[cfg(not(feature = "circular_buffer"))]
        WaterBuffer {
            data: other_data,
            start_pos: 0,
            filled_data_length: other_len,
        }
    }

    /// Splits the buffer, returning the front portion
    #[inline]
    pub fn split_to(&mut self, at: usize) -> WaterBuffer<InnerType> {
        assert!(
            at <= self.len(),
            "split_to out of bounds: {:?} <= {:?}",
            at,
            self.len(),
        );

        let mut front_data = Vec::with_capacity(at);
        if at > 0 {
            unsafe {
                front_data.set_len(at);
                ptr::copy_nonoverlapping(
                    self.data.as_ptr().add(self.start_pos),
                    front_data.as_mut_ptr(),
                    at
                );
            }
        }

        // Advance self
        self.start_pos += at;
        self.filled_data_length -= at;

        #[cfg(feature = "circular_buffer")]
        {
            WaterBuffer {
                data: front_data,
                start_pos: 0,
                filled_data_length: at,
                circular_position: None,
            }
        }

        #[cfg(not(feature = "circular_buffer"))]
        WaterBuffer {
            data: front_data,
            start_pos: 0,
            filled_data_length: at,
        }
    }

    /// Truncates the buffer to the specified length
    #[inline]
    pub fn truncate(&mut self, len: usize) {
        if len < self.filled_data_length {
            self.filled_data_length = len;
        }
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

impl Default for WaterBuffer<InnerType> {
    fn default() -> Self {
        Self::new()
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

impl Clone for WaterBuffer<InnerType> {
    fn clone(&self) -> Self {
        let mut new_data = Vec::with_capacity(self.filled_data_length);
        if self.filled_data_length > 0 {
            unsafe {
                new_data.set_len(self.filled_data_length);
                ptr::copy_nonoverlapping(
                    self.data.as_ptr().add(self.start_pos),
                    new_data.as_mut_ptr(),
                    self.filled_data_length
                );
            }
        }

        #[cfg(feature = "circular_buffer")]
        {
            WaterBuffer {
                data: new_data,
                start_pos: 0,
                filled_data_length: self.filled_data_length,
                circular_position: None,
            }
        }

        #[cfg(not(feature = "circular_buffer"))]
        WaterBuffer {
            data: new_data,
            start_pos: 0,
            filled_data_length: self.filled_data_length,
        }
    }
}