use std::alloc::{alloc, dealloc, realloc, Layout};
use std::ops::{Index, IndexMut, Range, RangeFull};
use std::ptr;

type InnerType = u8;

pub struct WaterBuffer<T> {
    cap: usize,
    start_pos: usize,
    pointer: *mut T,
    iterator_pos: usize,
    filled_data_length: usize,
}

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

impl Iterator for WaterBuffer<InnerType> {
    type Item = InnerType;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.iterator_pos ;
        if current + 1 > self.filled_data_length {
            self.iterator_pos = 0;
            return None;
        }
        self.iterator_pos += 1;
        Some(self[current])
    }
}

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
        unsafe {
            std::slice::from_raw_parts_mut(self.pointer.add(idx.start), idx.end - idx.start)
        }
    }
}

impl<T> Index<RangeFull> for WaterBuffer<T> {
    type Output = [T];
    fn index(&self, _idx: RangeFull) -> &Self::Output {
        unsafe {
            std::slice::from_raw_parts(
                self.pointer.add(self.start_pos),
                self.filled_data_length,
            )
        }
    }
}

impl<T> IndexMut<RangeFull> for WaterBuffer<T> {
    fn index_mut(&mut self, _idx: RangeFull) -> &mut Self::Output {
        unsafe {
            std::slice::from_raw_parts_mut(
                self.pointer.add(self.start_pos),
                self.filled_data_length,
            )
        }
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

impl WaterBuffer<InnerType> {
    pub fn with_capacity(cap: usize) -> WaterBuffer<InnerType> {
        // ✅ FIX: Use correct type size
        let layout = Layout::array::<InnerType>(cap).unwrap();
        let first_element_pointer = unsafe { alloc(layout) } as *mut InnerType;
        WaterBuffer {
            cap,
            pointer: first_element_pointer,
            start_pos: 0,
            iterator_pos: 0,
            filled_data_length: 0,
        }
    }

    #[inline(always)]
    pub fn expand(&mut self, n: usize) {
        // ✅ FIX: Update capacity after realloc
        let new_pointer = unsafe {
            realloc(
                self.pointer as *mut u8,
                Layout::array::<InnerType>(self.cap).unwrap(),
                n,
            )
        } as *mut InnerType;
        self.pointer = new_pointer;
        self.cap = n; // ✅ CRITICAL: Update capacity!
    }

    #[inline(always)]
    const fn ap_size(&self, len: usize) -> usize {
        let re = self.cap + (self.cap / 2);
        if len > re {
            return len;
        }
        re
    }

    #[inline(always)]
    pub fn extend_from_slice(&mut self, slice: &[u8]) {
        // ✅ FIX: Check filled_data_length, not start_pos
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

    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.filled_data_length
    }

    #[inline(always)]
    pub const fn reset(&mut self) {
        self.filled_data_length = 0;
        self.start_pos = 0;
        self.iterator_pos = 0;
    }

    #[inline]
    pub fn push(&mut self, item: InnerType) {
        // ✅ FIX: Check capacity properly
        if self.filled_data_length >= self.cap {
            self.expand(self.ap_size(self.filled_data_length + 1));
        }
        unsafe {
            // ✅ FIX: Write to correct position (filled_data_length)
            ptr::copy_nonoverlapping(
                &item,
                self.pointer.add(self.filled_data_length),
                1,
            );
        }
        // ✅ FIX: Increment length!
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


#[cfg(test)]
mod safety_tests {
    use super::WaterBuffer;

    // ============================================
    // 1. MEMORY SAFETY TESTS
    // ============================================

    #[test]
    fn test_no_use_after_free() {
        let mut buf = WaterBuffer::with_capacity(100);
        buf.push(42);
        buf.push(43);
        assert_eq!(buf[0], 42);
        assert_eq!(buf[1], 43);
        // Buffer should be safely dropped here
    }

    #[test]
    fn test_multiple_reallocations() {
        let mut buf = WaterBuffer::with_capacity(2);
        for i in 0..10000 {
            buf.push((i % 256) as u8);
        }
        assert_eq!(buf.len(), 10000);
        // Verify data integrity after many reallocations
        for i in 0..10000 {
            assert_eq!(buf[i], (i % 256) as u8);
        }
    }

    #[test]
    fn test_clear_and_reuse() {
        let mut buf = WaterBuffer::with_capacity(100);

        // First use
        for i in 0..50 {
            buf.push(i);
        }
        assert_eq!(buf.len(), 50);

        // Clear and reuse
        buf.clear();
        assert_eq!(buf.len(), 0);

        // Second use
        for i in 100..150 {
            buf.push(i);
        }
        assert_eq!(buf.len(), 50);
        assert_eq!(buf[0], 100);
    }

    #[test]
    fn test_extend_from_slice_reallocation() {
        let mut buf = WaterBuffer::with_capacity(10);
        let large_slice = vec![42u8; 1000];

        buf.extend_from_slice(&large_slice);
        assert_eq!(buf.len(), 1000);

        // Verify all data was copied correctly
        for i in 0..1000 {
            assert_eq!(buf[i], 42);
        }
    }

    // ============================================
    // 2. BOUNDS CHECKING TESTS
    // ============================================

    #[test]
    #[should_panic(expected = "Index out of bounds")]
    fn test_index_out_of_bounds() {
        let buf = WaterBuffer::with_capacity(10);
        let _ = buf[0]; // Should panic - buffer is empty
    }

    #[test]
    #[should_panic(expected = "Index out of bounds")]
    fn test_index_beyond_length() {
        let mut buf = WaterBuffer::with_capacity(10);
        buf.push(42);
        let _ = buf[5]; // Should panic - only 1 element
    }

    #[test]
    #[should_panic(expected = "Range out of bounds")]
    fn test_range_out_of_bounds() {
        let mut buf = WaterBuffer::with_capacity(10);
        buf.push(42);
        buf.push(43);
        let _ = &buf[0..10]; // Should panic - only 2 elements
    }

    #[test]
    #[should_panic(expected = "Range out of bounds")]
    fn test_invalid_range() {
        let mut buf = WaterBuffer::with_capacity(10);
        buf.push(42);
        buf.push(43);
        let _ = &buf[2..1]; // Should panic - start > end
    }

    // ============================================
    // 3. DATA INTEGRITY TESTS
    // ============================================

    #[test]
    fn test_data_integrity_after_growth() {
        let mut buf = WaterBuffer::with_capacity(4);

        // Fill initial capacity
        for i in 0..4 {
            buf.push(i);
        }

        // Force reallocation
        for i in 4..100 {
            buf.push(i);
        }

        // Verify all data is intact
        for i in 0..100 {
            assert_eq!(buf[i], i as u8, "Data corruption at index {}", i);
        }
    }

    #[test]
    fn test_extend_preserves_existing_data() {
        let mut buf = WaterBuffer::with_capacity(10);

        buf.push(1);
        buf.push(2);
        buf.push(3);

        let slice = vec![4, 5, 6, 7, 8];
        buf.extend_from_slice(&slice);

        assert_eq!(buf[0], 1);
        assert_eq!(buf[1], 2);
        assert_eq!(buf[2], 3);
        assert_eq!(buf[3], 4);
        assert_eq!(buf[7], 8);
    }

    #[test]
    fn test_interleaved_operations() {
        let mut buf = WaterBuffer::with_capacity(10);

        buf.push(1);
        buf.extend_from_slice(&[2, 3, 4]);
        buf.push(5);
        buf.extend_from_slice(&[6, 7]);
        buf.push(8);

        assert_eq!(buf.len(), 8);
        for i in 0..8 {
            assert_eq!(buf[i], (i + 1) as u8);
        }
    }

    // ============================================
    // 4. EDGE CASES
    // ============================================

    #[test]
    fn test_zero_capacity() {
        let mut buf = WaterBuffer::with_capacity(0);
        buf.push(42); // Should trigger allocation
        assert_eq!(buf[0], 42);
    }

    #[test]
    fn test_empty_slice_extend() {
        let mut buf = WaterBuffer::with_capacity(10);
        buf.push(42);

        let empty: &[u8] = &[];
        buf.extend_from_slice(empty);

        assert_eq!(buf.len(), 1);
        assert_eq!(buf[0], 42);
    }

    #[test]
    fn test_very_large_single_extend() {
        let mut buf = WaterBuffer::with_capacity(10);
        let large_data = vec![77u8; 1_000_000];

        buf.extend_from_slice(&large_data);

        assert_eq!(buf.len(), 1_000_000);
        assert_eq!(buf[0], 77);
        assert_eq!(buf[999_999], 77);
    }

    #[test]
    fn test_repeated_clear() {
        let mut buf = WaterBuffer::with_capacity(100);

        for _ in 0..1000 {
            buf.push(42);
            buf.push(43);
            assert_eq!(buf.len(), 2);
            buf.clear();
            assert_eq!(buf.len(), 0);
        }
    }

    // ============================================
    // 5. ITERATOR SAFETY TESTS
    // ============================================

    #[test]
    fn test_iterator_basic() {
        let mut buf = WaterBuffer::with_capacity(10);
        for i in 0..5 {
            buf.push(i * 10);
        }


        let collected: Vec<u8> = buf.into_iter().collect();
        println!("{:?}", collected);
        assert_eq!(collected, vec![0, 10, 20, 30, 40]);
    }

    #[test]
    fn test_iterator_empty_buffer() {
        let buf = WaterBuffer::with_capacity(10);
        let collected: Vec<u8> = buf.into_iter().collect();
        assert_eq!(collected.len(), 0);
    }

    // ============================================
    // 6. SLICE OPERATIONS TESTS
    // ============================================

    #[test]
    fn test_full_slice_access() {
        let mut buf = WaterBuffer::with_capacity(10);
        buf.extend_from_slice(&[1, 2, 3, 4, 5]);

        let slice = &buf[..];
        assert_eq!(slice, &[1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_partial_slice_access() {
        let mut buf = WaterBuffer::with_capacity(10);
        buf.extend_from_slice(&[1, 2, 3, 4, 5]);

        let slice = &buf[1..4];
        assert_eq!(slice, &[2, 3, 4]);
    }

    #[test]
    fn test_mutable_slice_modification() {
        let mut buf = WaterBuffer::with_capacity(10);
        buf.extend_from_slice(&[1, 2, 3, 4, 5]);

        {
            let slice = &mut buf[1..4];
            slice[0] = 99;
            slice[1] = 88;
            slice[2] = 77;
        }

        assert_eq!(buf[0], 1);
        assert_eq!(buf[1], 99);
        assert_eq!(buf[2], 88);
        assert_eq!(buf[3], 77);
        assert_eq!(buf[4], 5);
    }

    // ============================================
    // 7. CONCURRENT/STRESS TESTS
    // ============================================

    #[test]
    fn test_stress_many_operations() {
        let mut buf = WaterBuffer::with_capacity(100);

        for round in 0..100 {
            // Push phase
            for i in 0..1000 {
                buf.push((i % 256) as u8);
            }

            // Extend phase
            let chunk = vec![(round % 256) as u8; 500];
            buf.extend_from_slice(&chunk);

            // Verify length
            assert_eq!(buf.len(), 1500);

            // Clear
            buf.clear();
            assert_eq!(buf.len(), 0);
        }
    }

    #[test]
    fn test_alternating_grow_clear() {
        let mut buf = WaterBuffer::with_capacity(2);

        for _ in 0..1000 {
            // Grow
            for i in 0..100 {
                buf.push(i);
            }
            assert_eq!(buf.len(), 100);

            // Clear
            buf.clear();
            assert_eq!(buf.len(), 0);
        }
    }

    // ============================================
    // 8. MEMORY LEAK DETECTION (requires manual inspection)
    // ============================================

    #[test]
    fn test_no_memory_leak_on_drop() {
        // Run with: cargo test --release -- --nocapture
        // Monitor with: valgrind, instruments, or heaptrack

        for _ in 0..10000 {
            let mut buf = WaterBuffer::with_capacity(1000);
            for i in 0..1000 {
                buf.push(i as u8);
            }
            // Buffer dropped here - should free memory
        }

        // If this completes without OOM, basic drop safety is confirmed
    }

    // ============================================
    // 9. SPECIFIC BUG REGRESSION TESTS
    // ============================================

    #[test]
    fn test_push_updates_length() {
        // Regression: Original bug where push didn't increment length
        let mut buf = WaterBuffer::with_capacity(10);

        buf.push(1);
        assert_eq!(buf.len(), 1);

        buf.push(2);
        assert_eq!(buf.len(), 2);

        buf.push(3);
        assert_eq!(buf.len(), 3);
    }

    #[test]
    fn test_expand_updates_capacity() {
        // Regression: Original bug where expand didn't update cap
        let mut buf = WaterBuffer::with_capacity(2);

        // Force multiple expansions
        for i in 0..100 {
            buf.push(i);
        }

        // Should not crash and all data should be accessible
        for i in 0..100 {
            assert_eq!(buf[i], i as u8);
        }
    }

    #[test]
    fn test_extend_checks_correct_length() {
        // Regression: Original bug checking start_pos instead of filled_data_length
        let mut buf = WaterBuffer::with_capacity(10);

        buf.extend_from_slice(&[1, 2, 3, 4, 5]);
        assert_eq!(buf.len(), 5);

        // This should trigger reallocation
        let large_slice = vec![42u8; 100];
        buf.extend_from_slice(&large_slice);

        assert_eq!(buf.len(), 105);
    }
}

// ============================================
// 10. PROPERTY-BASED TESTING (with proptest)
// ============================================

#[cfg(test)]
mod property_tests {
    use super::WaterBuffer;

    // Uncomment if you add proptest dependency:
    // use proptest::prelude::*;

    // proptest! {
    //     #[test]
    //     fn test_arbitrary_pushes(data in prop::collection::vec(any::<u8>(), 0..10000)) {
    //         let mut buf = WaterBuffer::with_capacity(10);
    //         for &byte in &data {
    //             buf.push(byte);
    //         }
    //         assert_eq!(buf.len(), data.len());
    //         for (i, &byte) in data.iter().enumerate() {
    //             assert_eq!(buf[i], byte);
    //         }
    //     }
    // }
}