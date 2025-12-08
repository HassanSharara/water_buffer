mod not_expand;

#[cfg(all(test, not(feature = "do-not-expand")))]
mod tests {
    use super::super::*;

    // ============================================================================
    // BASIC EXPANSION TESTS
    // ============================================================================

    #[test]
    fn test_with_capacity() {
        let b = WaterBuffer::with_capacity(10);
        assert_eq!(b.cap, 10);
        assert_eq!(b.len(), 0);
        assert_eq!(b.start_pos, 0);
    }

    #[test]
    fn test_extend_within_capacity() {
        let mut b = WaterBuffer::with_capacity(10);
        b.extend_from_slice(b"hello");
        assert_eq!(&b[..], b"hello");
        assert_eq!(b.len(), 5);
        assert_eq!(b.cap, 10);
    }

    #[test]
    fn test_auto_expand() {
        let mut b = WaterBuffer::with_capacity(5);
        b.extend_from_slice(b"hello world"); // 11 bytes > 5 capacity
        assert_eq!(&b[..], b"hello world");
        assert_eq!(b.len(), 11);
        assert!(b.cap >= 11); // Should have expanded
    }

    #[test]
    fn test_expand_exact_capacity() {
        let mut b = WaterBuffer::with_capacity(5);
        b.extend_from_slice(b"ABCDE");
        assert_eq!(&b[..], b"ABCDE");
        assert_eq!(b.len(), 5);
        assert_eq!(b.cap, 5);
    }

    #[test]
    fn test_expand_multiple_times() {
        let mut b = WaterBuffer::with_capacity(2);
        b.extend_from_slice(b"AB");
        assert_eq!(b.cap, 2);

        b.extend_from_slice(b"CD");
        assert!(b.cap > 2);

        b.extend_from_slice(b"EFGHIJ");
        assert_eq!(&b[..], b"ABCDEFGHIJ");
        assert_eq!(b.len(), 10);
    }

    // ============================================================================
    // PUSH TESTS
    // ============================================================================

    #[test]
    fn test_push_single() {
        let mut b = WaterBuffer::with_capacity(5);
        b.push(b'A');
        assert_eq!(&b[..], b"A");
        assert_eq!(b.len(), 1);
    }

    #[test]
    fn test_push_multiple() {
        let mut b = WaterBuffer::with_capacity(3);
        b.push(b'A');
        b.push(b'B');
        b.push(b'C');
        assert_eq!(&b[..], b"ABC");
        assert_eq!(b.len(), 3);
    }

    #[test]
    fn test_push_with_auto_expand() {
        let mut b = WaterBuffer::with_capacity(2);
        b.push(b'A');
        b.push(b'B');
        b.push(b'C'); // Should expand
        assert_eq!(&b[..], b"ABC");
        assert_eq!(b.len(), 3);
        assert!(b.cap >= 3);
    }

    #[test]
    fn test_push_many() {
        let mut b = WaterBuffer::with_capacity(2);
        for byte in b"ABCDEFGHIJ" {
            b.push(*byte);
        }
        assert_eq!(&b[..], b"ABCDEFGHIJ");
        assert_eq!(b.len(), 10);
    }

    // ============================================================================
    // INDEXING TESTS
    // ============================================================================

    #[test]
    fn test_index_single() {
        let mut b = WaterBuffer::with_capacity(5);
        b.extend_from_slice(b"ABCDE");
        assert_eq!(b[0], b'A');
        assert_eq!(b[2], b'C');
        assert_eq!(b[4], b'E');
    }

    #[test]
    fn test_index_mut() {
        let mut b = WaterBuffer::with_capacity(5);
        b.extend_from_slice(b"ABCDE");
        b[0] = b'X';
        b[4] = b'Y';
        assert_eq!(&b[..], b"XBCDY");
    }

    #[test]
    fn test_range_indexing() {
        let mut b = WaterBuffer::with_capacity(10);
        b.extend_from_slice(b"0123456789");
        assert_eq!(&b[0..3], b"012");
        assert_eq!(&b[5..8], b"567");
        assert_eq!(&b[7..10], b"789");
    }

    #[test]
    fn test_range_mut_indexing() {
        let mut b = WaterBuffer::with_capacity(10);
        b.extend_from_slice(b"0123456789");
        b[2..5].copy_from_slice(b"XYZ");
        assert_eq!(&b[..], b"01XYZ56789");
    }

    #[test]
    fn test_full_slice() {
        let mut b = WaterBuffer::with_capacity(5);
        b.extend_from_slice(b"HELLO");
        assert_eq!(&b[..], b"HELLO");
    }

    #[test]
    #[should_panic(expected = "Index out of bounds")]
    fn test_index_out_of_bounds() {
        let mut b = WaterBuffer::with_capacity(5);
        b.extend_from_slice(b"ABC");
        let _ = b[5];
    }

    // ============================================================================
    // ITERATION TESTS
    // ============================================================================

    #[test]
    fn test_iter() {
        let mut b = WaterBuffer::with_capacity(5);
        b.extend_from_slice(b"ABCDE");

        let collected: Vec<u8> = b.iter().copied().collect();
        assert_eq!(collected, b"ABCDE");
    }

    #[test]
    fn test_iter_mut() {
        let mut b = WaterBuffer::with_capacity(5);
        b.extend_from_slice(b"ABCDE");

        for byte in b.iter_mut() {
            *byte = byte.to_ascii_lowercase();
        }

        assert_eq!(&b[..], b"abcde");
    }

    #[test]
    fn test_owned_iter() {
        let mut b = WaterBuffer::with_capacity(5);
        b.extend_from_slice(b"ABCDE");

        let collected: Vec<u8> = b.into_owned_iter().collect();
        assert_eq!(collected, b"ABCDE");
    }

    #[test]
    fn test_iter_empty() {
        let b = WaterBuffer::with_capacity(5);
        let collected: Vec<u8> = b.iter().copied().collect();
        assert_eq!(collected.len(), 0);
    }

    #[test]
    fn test_iter_after_expand() {
        let mut b = WaterBuffer::with_capacity(3);
        b.extend_from_slice(b"ABCDEFGH"); // Will expand

        let collected: Vec<u8> = b.iter().copied().collect();
        assert_eq!(collected, b"ABCDEFGH");
    }

    // ============================================================================
    // LENGTH AND CAPACITY TESTS
    // ============================================================================

    #[test]
    fn test_len() {
        let mut b = WaterBuffer::with_capacity(10);
        assert_eq!(b.len(), 0);

        b.extend_from_slice(b"ABC");
        assert_eq!(b.len(), 3);

        b.extend_from_slice(b"DEF");
        assert_eq!(b.len(), 6);
    }

    #[test]
    fn test_remaining() {
        let mut b = WaterBuffer::with_capacity(10);
        assert_eq!(b.remaining(), 0);

        b.extend_from_slice(b"HELLO");
        assert_eq!(b.remaining(), 5);
    }

    #[test]
    fn test_un_initialized_remaining() {
        let mut b = WaterBuffer::with_capacity(10);
        assert_eq!(b.un_initialized_remaining(), 10);

        b.extend_from_slice(b"ABCDE");
        assert_eq!(b.un_initialized_remaining(), 5);

        b.extend_from_slice(b"12345");
        assert_eq!(b.un_initialized_remaining(), 0);
    }

    #[test]
    fn test_un_initialized_after_expand() {
        let mut b = WaterBuffer::with_capacity(5);
        b.extend_from_slice(b"ABCDEFGH"); // Expands to at least 8

        let uninit = b.un_initialized_remaining();

         assert_eq!(uninit,0);
    }

    // ============================================================================
    // RESET AND CLEAR TESTS
    // ============================================================================

    #[test]
    fn test_reset() {
        let mut b = WaterBuffer::with_capacity(10);
        b.extend_from_slice(b"HELLO");
        assert_eq!(b.len(), 5);

        b.reset();
        assert_eq!(b.len(), 0);
        assert_eq!(b.start_pos, 0);

        b.extend_from_slice(b"WORLD");
        assert_eq!(&b[..], b"WORLD");
    }

    #[test]
    fn test_clear() {
        let mut b = WaterBuffer::with_capacity(10);
        b.extend_from_slice(b"HELLO");

        b.clear();
        assert_eq!(b.len(), 0);
    }

    #[test]
    fn test_clear_then_reuse() {
        let mut b = WaterBuffer::with_capacity(5);
        b.extend_from_slice(b"HELLO");
        let old_cap = b.cap;

        b.clear();
        b.extend_from_slice(b"WORLD");

        assert_eq!(&b[..], b"WORLD");
        assert_eq!(b.cap, old_cap); // Capacity should not change
    }

    // ============================================================================
    // ADVANCE TESTS
    // ============================================================================

    #[test]
    fn test_advance() {
        let mut b = WaterBuffer::with_capacity(10);
        b.extend_from_slice(b"ABCDEFGH");

        b.advance(3);
        assert_eq!(b.len(), 5);
        assert_eq!(b.start_pos, 3);
        assert_eq!(&b[..], b"DEFGH");
    }

    #[test]
    fn test_advance_partial() {
        let mut b = WaterBuffer::with_capacity(10);
        b.extend_from_slice(b"0123456789");

        b.advance(2);
        assert_eq!(&b[..], b"23456789");

        b.advance(3);
        assert_eq!(&b[..], b"56789");
    }

    #[test]
    fn test_advance_all() {
        let mut b = WaterBuffer::with_capacity(5);
        b.extend_from_slice(b"HELLO");

        b.advance(5);
        assert_eq!(b.len(), 0);
    }

    #[test]
    #[should_panic(expected = "Insufficient space to advance")]
    fn test_advance_too_much() {
        let mut b = WaterBuffer::with_capacity(5);
        b.extend_from_slice(b"ABC");
        b.advance(4);
    }

    #[test]
    fn test_advance_then_extend() {
        let mut b = WaterBuffer::with_capacity(10);
        b.extend_from_slice(b"ABCDE");
        b.advance(2);
        assert_eq!(&b[..], b"CDE");
        b.extend_from_slice(b"FGH");
        assert_eq!(&b[..], b"CDEFGH");
    }

    // ============================================================================
    // CHUNK_MUT AND ADVANCE_MUT TESTS
    // ============================================================================

    #[test]
    fn test_chunk_mut() {
        let mut b = WaterBuffer::with_capacity(10);
        b.extend_from_slice(b"ABCDE");

        let chunk = b.chunk_mut();
        assert_eq!(chunk.len(), 5);
    }

    #[test]
    fn test_chunk_mut_write() {
        let mut b = WaterBuffer::with_capacity(10);
        b.extend_from_slice(b"HELLO");
        let chunk = b.chunk_mut();
        chunk[0] = b'X';
        chunk[1] = b'Y';
        b.advance_mut(2);
        assert_eq!(&b[..], b"HELLOXY");
    }

    #[test]
    fn test_advance_mut() {
        let mut b = WaterBuffer::with_capacity(10);
        b.extend_from_slice(b"ABC");
        assert_eq!(b.len(), 3);

        b.advance_mut(2);
        assert_eq!(b.len(), 5);
    }

    // ============================================================================
    // EDGE CASES
    // ============================================================================

    #[test]
    fn test_empty_buffer() {
        let b = WaterBuffer::with_capacity(5);
        assert_eq!(b.len(), 0);
        assert_eq!(&b[..], b"");
    }

    #[test]
    fn test_single_byte_capacity() {
        let mut b = WaterBuffer::with_capacity(1);
        b.push(b'A');
        assert_eq!(&b[..], b"A");

        b.push(b'B'); // Should expand
        assert_eq!(&b[..], b"AB");
    }

    #[test]
    fn test_extend_empty_slice() {
        let mut b = WaterBuffer::with_capacity(5);
        b.extend_from_slice(b"");
        assert_eq!(b.len(), 0);
    }

    #[test]
    fn test_large_initial_capacity() {
        let mut b = WaterBuffer::with_capacity(1000);
        b.extend_from_slice(b"Hello");
        assert_eq!(&b[..], b"Hello");
        assert_eq!(b.cap, 1000);
    }

    #[test]
    fn test_zero_capacity_then_push() {
        let mut b = WaterBuffer::with_capacity(0);
        b.push(b'A'); // Should expand
        assert_eq!(&b[..], b"A");
        assert!(b.cap > 0);
    }

    // ============================================================================
    // EXPANSION STRATEGY TESTS
    // ============================================================================

    #[test]
    fn test_expansion_growth() {
        let mut b = WaterBuffer::with_capacity(4);
        b.extend_from_slice(b"ABCD");
        let cap1 = b.cap;

        b.extend_from_slice(b"E"); // Should expand
        let cap2 = b.cap;

        assert!(cap2 > cap1);
        assert_eq!(&b[..], b"ABCDE");
    }

    #[test]
    fn test_ap_size() {
        let b = WaterBuffer::with_capacity(10);
        let new_size = b.ap_size(15);
        assert!(new_size >= 15);
    }

    #[test]
    fn test_expand_preserves_data() {
        let mut b = WaterBuffer::with_capacity(5);
        b.extend_from_slice(b"HELLO");

        b.extend_from_slice(b" WORLD"); // Forces expansion
        assert_eq!(&b[..], b"HELLO WORLD");
    }

    // ============================================================================
    // COMPLEX SCENARIOS
    // ============================================================================

    #[test]
    fn test_multiple_operations() {
        let mut b = WaterBuffer::with_capacity(5);

        b.extend_from_slice(b"ABC");
        assert_eq!(&b[..], b"ABC");

        b.push(b'D');
        assert_eq!(&b[..], b"ABCD");

        b.advance(2);
        assert_eq!(&b[..], b"CD");

        b.extend_from_slice(b"EFGH");
        assert_eq!(&b[..], b"CDEFGH");
    }

    #[test]
    fn test_reset_and_reuse() {
        let mut b = WaterBuffer::with_capacity(10);

        b.extend_from_slice(b"FIRST");
        assert_eq!(&b[..], b"FIRST");

        b.reset();
        b.extend_from_slice(b"SECOND");
        assert_eq!(&b[..], b"SECOND");
    }

    #[test]
    fn test_advance_reset_extend() {
        let mut b = WaterBuffer::with_capacity(10);
        b.extend_from_slice(b"ABCDEFGH");

        b.advance(3);
        assert_eq!(&b[..], b"DEFGH");

        b.reset();
        b.extend_from_slice(b"123");
        assert_eq!(&b[..], b"123");
    }

    #[test]
    fn test_interleaved_push_extend() {
        let mut b = WaterBuffer::with_capacity(10);

        b.push(b'A');
        b.extend_from_slice(b"BC");
        b.push(b'D');
        b.extend_from_slice(b"EF");

        assert_eq!(&b[..], b"ABCDEF");
    }

    #[test]
    fn test_large_extend() {
        let mut b = WaterBuffer::with_capacity(5);
        let large_data = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";

        b.extend_from_slice(large_data);
        assert_eq!(&b[..], large_data);
        assert!(b.cap >= large_data.len());
    }

    #[test]
    fn test_gradual_growth() {
        let mut b = WaterBuffer::with_capacity(2);

        for i in 0..20 {
            b.push(b'A' + (i % 26));
        }

        assert_eq!(b.len(), 20);
        assert!(b.cap >= 20);
    }
}