#[cfg(all(test,feature = "circular_buffer"))]
mod tests {
    use super::super::super::*;

    // ============================================================================
    // BASIC WRAPPING TESTS
    // ============================================================================

    #[test]
    fn test_basic_wrap() {
        let mut b = WaterBuffer::with_capacity(10);
        b.extend_from_slice(b"hello world"); // 11 bytes → wraps
        let s = &b[..];
        assert_eq!(s, b"dello worl"); // the first 10 bytes
    }

    #[test]
    fn test_wrap_overwrite() {
        let mut b = WaterBuffer::with_capacity(8);
        b.extend_from_slice(b"12345678");
        assert_eq!(&b[..], b"12345678");
        b.extend_from_slice(b"ABCD");
        assert_eq!(&b[..], b"ABCD5678");
    }

    #[test]
    fn test_multiple_wraps() {
        let mut b = WaterBuffer::with_capacity(5);
        b.extend_from_slice(b"ABCDE");
        assert_eq!(&b[..], b"ABCDE");
        b.extend_from_slice(b"12");
        assert_eq!(&b[..], b"12CDE");
        b.extend_from_slice(b"XYZ");
        assert_eq!(&b[..], b"12XYZ");
    }

    #[test]
    fn test_exact_capacity_write() {
        let mut b = WaterBuffer::with_capacity(5);
        b.extend_from_slice(b"ABCDE");
        assert_eq!(&b[..], b"ABCDE");
        assert_eq!(b.len(), 5);
        assert_eq!(b.start_pos, 0);
    }

    #[test]
    fn test_wrap_exactly_once() {
        let mut b = WaterBuffer::with_capacity(4);
        b.extend_from_slice(b"ABCD");
        b.extend_from_slice(b"EFGH");
        assert_eq!(&b[..], b"EFGH");
        assert_eq!(b.start_pos, 0);
    }

    // ============================================================================
    // PUSH TESTS
    // ============================================================================

    #[test]
    fn test_push_with_wrap() {
        let mut b = WaterBuffer::with_capacity(3);
        b.push(b'A');
        b.push(b'B');
        b.push(b'C');
        assert_eq!(&b[..], b"ABC");

        b.push(b'X'); // Should wrap
        assert_eq!(&b[..], b"XBC");
    }

    #[test]
    fn test_push_multiple_wraps() {
        let mut b = WaterBuffer::with_capacity(3);
        for byte in b"ABCDEFGH" {
            b.push(*byte);
        }
        // Last 3 bytes should remain
        assert_eq!(&b[..], b"GHF");
    }

    // ============================================================================
    // INDEXING TESTS
    // ============================================================================

    #[test]
    fn test_indexing_after_wrap() {
        let mut b = WaterBuffer::with_capacity(4);
        b.extend_from_slice(b"ABCD");
        b.extend_from_slice(b"Z");

        assert_eq!(b[0], b'Z');
        assert_eq!(b[1], b'B');
        assert_eq!(b[2], b'C');
        assert_eq!(b[3], b'D');
    }

    #[test]
    fn test_index_mut() {
        let mut b = WaterBuffer::with_capacity(4);
        b.extend_from_slice(b"ABCD");
        b[0] = b'X';
        b[3] = b'Y';
        assert_eq!(&b[..], b"XBCY");
    }

    #[test]
    fn test_range_indexing() {
        let mut b = WaterBuffer::with_capacity(8);
        b.extend_from_slice(b"12345678");
        assert_eq!(&b[2..5], b"345");
        assert_eq!(&b[0..3], b"123");
    }

    #[test]
    fn test_range_mut_indexing() {
        let mut b = WaterBuffer::with_capacity(8);
        b.extend_from_slice(b"12345678");
        b[2..5].copy_from_slice(b"XYZ");
        assert_eq!(&b[..], b"12XYZ678");
    }

    // ============================================================================
    // ITERATION TESTS
    // ============================================================================

    #[test]
    fn test_iter_after_wrap() {
        let mut b = WaterBuffer::with_capacity(6);
        b.extend_from_slice(b"123456");
        b.extend_from_slice(b"XYZ");

        let collected: Vec<u8> = b.iter().copied().collect();
        assert_eq!(collected, b"XYZ456");
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
        let mut b = WaterBuffer::with_capacity(4);
        b.extend_from_slice(b"ABCD");

        let collected: Vec<u8> = b.into_owned_iter().collect();
        assert_eq!(collected, b"ABCD");
    }

    #[test]
    fn test_owned_iter_after_wrap() {
        let mut b = WaterBuffer::with_capacity(4);
        b.extend_from_slice(b"ABCDEFGH");

        let collected: Vec<u8> = b.into_owned_iter().collect();
        assert_eq!(collected, b"EFGH");
    }

    // ============================================================================
    // START_POS AND LENGTH TESTS
    // ============================================================================

    #[test]
    fn test_start_pos_updates() {
        let mut b = WaterBuffer::with_capacity(6);
        b.extend_from_slice(b"HELLO!");
        assert_eq!(b.start_pos, 0);

        b.extend_from_slice(b"OK");
        assert_eq!(b.circular_position, Some(2));
    }

    #[test]
    fn test_length_tracking() {
        let mut b = WaterBuffer::with_capacity(5);
        assert_eq!(b.len(), 0);

        b.extend_from_slice(b"ABC");
        assert_eq!(b.len(), 3);

        b.extend_from_slice(b"DEF"); // Wraps
        assert_eq!(b.len(), 5); // Capped at capacity
    }

    // ============================================================================
    // RESET AND CLEAR TESTS
    // ============================================================================

    #[test]
    fn test_reset() {
        let mut b = WaterBuffer::with_capacity(5);
        b.extend_from_slice(b"ABCDE");
        b.reset();

        assert_eq!(b.len(), 0);
        assert_eq!(b.start_pos, 0);
    }

    #[test]
    fn test_clear() {
        let mut b = WaterBuffer::with_capacity(5);
        b.extend_from_slice(b"ABCDE");
        b.clear();

        assert_eq!(b.len(), 0);
        assert_eq!(b.start_pos, 0);
    }

    #[test]
    fn test_reset_after_wrap() {
        let mut b = WaterBuffer::with_capacity(5);
        b.extend_from_slice(b"ABCDEFGH");
        b.reset();

        b.extend_from_slice(b"123");
        assert_eq!(&b[..], b"123");
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
    fn test_advance_all() {
        let mut b = WaterBuffer::with_capacity(5);
        b.extend_from_slice(b"ABCDE");

        b.advance(5);
        assert_eq!(b.len(), 0);
    }

    #[test]
    #[should_panic(expected = "Insufficient space to advance")]
    fn test_advance_too_much() {
        let mut b = WaterBuffer::with_capacity(5);
        b.extend_from_slice(b"ABC");
        b.advance(4); // Should panic
    }

    // ============================================================================
    // CHUNK_MUT AND ADVANCE_MUT TESTS
    // ============================================================================

    #[test]
    fn test_chunk_mut() {
        let mut b = WaterBuffer::with_capacity(10);
        b.extend_from_slice(b"ABCDE");

        let chunk = b.chunk_mut();
        assert_eq!(chunk.len(), 5); // 10 - 5 = 5 remaining
    }

    #[test]
    fn test_advance_mut() {
        let mut b = WaterBuffer::with_capacity(10);
        b.extend_from_slice(b"ABCDE");

        let chunk = b.chunk_mut();
        chunk[0] = b'X';
        chunk[1] = b'Y';
        b.advance_mut(2);

        assert_eq!(b.len(), 7);
        assert_eq!(&b[..], b"ABCDEXY");
    }

    // ============================================================================
    // UN_INITIALIZED_REMAINING TESTS
    // ============================================================================

    #[test]
    fn test_un_initialized_remaining() {
        let mut b = WaterBuffer::with_capacity(10);
        assert_eq!(b.un_initialized_remaining(), 10);

        b.extend_from_slice(b"ABCDE");
        assert_eq!(b.un_initialized_remaining(), 5);
    }

    #[test]
    fn test_un_initialized_remaining_after_wrap() {
        let mut b = WaterBuffer::with_capacity(5);
        b.extend_from_slice(b"ABCDEFGH"); // Wraps

        // After wrap, should have capacity available
        let remaining = b.un_initialized_remaining();
        assert!(remaining > 0);
    }

    // ============================================================================
    // REMAINING TESTS
    // ============================================================================

    #[test]
    fn test_remaining() {
        let mut b = WaterBuffer::with_capacity(10);
        assert_eq!(b.remaining(), 0);

        b.extend_from_slice(b"ABCDE");
        assert_eq!(b.remaining(), 5);
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

        b.push(b'B');
        assert_eq!(&b[..], b"B");
    }

    #[test]
    fn test_extend_empty_slice() {
        let mut b = WaterBuffer::with_capacity(5);
        b.extend_from_slice(b"");
        assert_eq!(b.len(), 0);
    }

    #[test]
    fn test_full_slice_after_wrap() {
        let mut b = WaterBuffer::with_capacity(5);
        b.extend_from_slice(b"aaaaa");
        b.extend_from_slice(b"bbb");
        assert_eq!(&b[..], b"bbbaa");
    }

    #[test]
    fn test_wrap_with_very_large_write() {
        let mut b = WaterBuffer::with_capacity(5);
        b.extend_from_slice(b"0123456789ABCDEF"); // 16 bytes into 5-byte buffer
        // Should contain last 5 bytes written
        assert_eq!(b.len(), 5);
    }

    #[test]
    fn test_alternating_push_and_wrap() {
        let mut b = WaterBuffer::with_capacity(3);
        b.push(b'A');
        b.push(b'B');
        b.push(b'C');
        assert_eq!(&b[..], b"ABC");

        b.push(b'D');
        assert_eq!(&b[..], b"DBC");

        b.push(b'E');
        assert_eq!(&b[..], b"DEC");
        b.push(b'F');
        assert_eq!(&b[..], b"DEF");
    }

    // ============================================================================
    // COMPLEX SCENARIOS
    // ============================================================================

    #[test]
    fn test_wrap_reset_wrap() {
        let mut b = WaterBuffer::with_capacity(4);
        b.extend_from_slice(b"ABCDEFGH");
        assert_eq!(&b[..], b"EFGH");

        b.reset();
        b.extend_from_slice(b"1234");
        assert_eq!(&b[..], b"1234");

        b.extend_from_slice(b"XY");
        assert_eq!(&b[..], b"XY34");
    }

    #[test]
    fn test_advance_then_wrap() {
        let mut b = WaterBuffer::with_capacity(8);
        b.extend_from_slice(b"ABCDEFGH");
        b.advance(4);
        assert_eq!(&b[..], b"EFGH");

        b.extend_from_slice(b"12345");
        // After advance, we have 4 bytes, adding 5 more should wrap
        assert_eq!(b.len(), 8);
    }
}





#[cfg(all(test, feature = "circular_buffer"))]
mod comprehensive_tests {
    use super::super::super::*;

    // ============================================================================
    // WRAPPING EDGE CASES
    // ============================================================================

    #[test]
    fn test_wrap_exactly_2x_capacity() {
        let mut b = WaterBuffer::with_capacity(4);
        b.extend_from_slice(b"ABCDEFGH"); // Exactly 2x capacity
        assert_eq!(&b[..], b"EFGH");
        assert_eq!(b.len(), 4);
        assert_eq!(b.circular_position, Some(0));
    }

    #[test]
    fn test_wrap_exactly_3x_capacity() {
        let mut b = WaterBuffer::with_capacity(3);
        b.extend_from_slice(b"ABCDEFGHI"); // Exactly 3x capacity
        assert_eq!(&b[..], b"GHI");
        assert_eq!(b.len(), 3);
    }

    #[test]
    fn test_wrap_with_non_zero_start_pos() {
        let mut b = WaterBuffer::with_capacity(8);
        b.extend_from_slice(b"ABCD");
        b.advance(2); // start_pos = 2, remaining = "CD"
        assert_eq!(b.start_pos, 2);

        b.extend_from_slice(b"EFGHIJKL"); // Add 8 more bytes
        // Total filled would be 10, capacity is 8
        assert_eq!(b.len(), 8);
    }

    #[test]
    fn test_multiple_complete_wraps() {
        let mut b = WaterBuffer::with_capacity(3);
        b.extend_from_slice(b"ABC");
        assert_eq!(&b[..], b"ABC");

        b.extend_from_slice(b"123456789"); // 3x capacity
        assert_eq!(&b[..], b"789");
        assert_eq!(b.len(), 3);
    }

    #[test]
    fn test_wrap_then_partial_write() {
        let mut b = WaterBuffer::with_capacity(5);
        b.extend_from_slice(b"ABCDEFGH"); // Wrap
        assert_eq!(&b[..], b"FGHDE");

        b.extend_from_slice(b"12"); // Partial write after wrap
        assert_eq!(&b[..], b"FGH12");
    }

    // ============================================================================
    // ADVANCE WITH WRAPPING
    // ============================================================================

    #[test]
    fn test_advance_after_wrap() {
        let mut b = WaterBuffer::with_capacity(6);
        b.extend_from_slice(b"ABCDEFGHIJ"); // Wrap occurs
        assert_eq!(b.len(), 6);

        b.advance(3);
        assert_eq!(b.len(), 3);
        assert_eq!(b.start_pos, 3);
    }

    #[test]
    fn test_multiple_advances() {
        let mut b = WaterBuffer::with_capacity(10);
        b.extend_from_slice(b"ABCDEFGH");

        b.advance(2);
        assert_eq!(b.len(), 6);
        assert_eq!(b.start_pos, 2);

        b.advance(3);
        assert_eq!(b.len(), 3);
        assert_eq!(b.start_pos, 5);

        b.advance(3);
        assert_eq!(b.len(), 0);
        assert_eq!(b.start_pos, 8);
    }

    #[test]
    fn test_advance_then_extend_after_wrap() {
        let mut b = WaterBuffer::with_capacity(6);
        b.extend_from_slice(b"ABCDEFGHIJ");

        b.advance(4);
        assert_eq!(b.len(), 2);

        b.extend_from_slice(b"12345");
        assert_eq!(b.len(), 6); // Should be at capacity
    }

    #[test]
    fn test_advance_to_exact_length() {
        let mut b = WaterBuffer::with_capacity(5);
        b.extend_from_slice(b"ABCDE");

        b.advance(5);
        assert_eq!(b.len(), 0);
        assert_eq!(b.start_pos, 5);
    }

    // ============================================================================
    // RANGE OPERATIONS AFTER WRAP
    // ============================================================================

    #[test]
    fn test_range_indexing_after_wrap() {
        let mut b = WaterBuffer::with_capacity(8);
        b.extend_from_slice(b"ABCDEFGHIJKL");
        // Buffer should contain last 8 bytes: "EFGHIJKL"

        let slice = &b[2..5];
        assert_eq!(slice.len(), 3);
    }

    #[test]
    fn test_range_mut_after_wrap() {
        let mut b = WaterBuffer::with_capacity(6);
        b.extend_from_slice(b"ABCDEFGH");

        b[1..4].copy_from_slice(b"XYZ");
        let result = &b[..];
        assert_eq!(result.len(), 6);
    }

    #[test]
    fn test_full_range_after_multiple_wraps() {
        let mut b = WaterBuffer::with_capacity(4);
        b.extend_from_slice(b"AAAABBBBCCCC"); // Multiple wraps

        let full = &b[..];
        assert_eq!(full.len(), 4);
    }

    // ============================================================================
    // CHUNK_MUT EDGE CASES
    // ============================================================================

    #[test]
    fn test_chunk_mut_when_full() {
        let mut b = WaterBuffer::with_capacity(5);
        b.extend_from_slice(b"ABCDE");

        let chunk = b.chunk_mut();
        assert_eq!(chunk.len(), 0); // No space remaining
    }

    #[test]
    fn test_chunk_mut_after_wrap() {
        let mut b = WaterBuffer::with_capacity(6);
        b.extend_from_slice(b"ABCDEFGH"); // Wraps

        let chunk = b.chunk_mut();
        // After wrap, circular_position should be set
        assert_eq!(chunk.len(), 0);
    }

    #[test]
    fn test_multiple_chunk_mut_advance_mut_cycles() {
        let mut b = WaterBuffer::with_capacity(10);

        // First cycle
        let chunk1 = b.chunk_mut();
        chunk1[0] = b'A';
        chunk1[1] = b'B';
        b.advance_mut(2);
        assert_eq!(b.len(), 2);

        // Second cycle
        let chunk2 = b.chunk_mut();
        chunk2[0] = b'C';
        chunk2[1] = b'D';
        b.advance_mut(2);
        assert_eq!(b.len(), 4);

        assert_eq!(&b[..], b"ABCD");
    }

    #[test]
    fn test_chunk_mut_with_start_pos() {
        let mut b = WaterBuffer::with_capacity(10);
        b.extend_from_slice(b"ABCD");
        b.advance(2);

        let chunk = b.chunk_mut();
        assert!(chunk.len() > 0);
    }

    // ============================================================================
    // ITERATION EDGE CASES
    // ============================================================================

    #[test]
    fn test_iter_with_non_zero_start_pos() {
        let mut b = WaterBuffer::with_capacity(10);
        b.extend_from_slice(b"ABCDEFGH");
        b.advance(3);
        let collected: Vec<u8> = b.iter().copied().collect();
        assert_eq!(collected, b"DEFGH");
    }

    #[test]
    fn test_iter_mut_after_wrap() {
        let mut b = WaterBuffer::with_capacity(5);
        b.extend_from_slice(b"ABCDEFGH");

        for byte in b.iter_mut() {
            *byte = b'X';
        }

        let collected: Vec<u8> = b.iter().copied().collect();
        assert!(collected.iter().all(|&b| b == b'X'));
    }

    #[test]
    fn test_iter_empty_buffer() {
        let b = WaterBuffer::with_capacity(5);
        let collected: Vec<u8> = b.iter().copied().collect();
        assert_eq!(collected.len(), 0);
    }

    #[test]
    fn test_owned_iter_with_advance() {
        let mut b = WaterBuffer::with_capacity(8);
        b.extend_from_slice(b"ABCDEFGH");
        b.advance(3);

        let collected: Vec<u8> = b.into_owned_iter().collect();
        assert_eq!(collected, b"DEFGH");
    }

    // ============================================================================
    // CAPACITY ZERO
    // ============================================================================

    #[test]
    fn test_zero_capacity_creation() {
        let b = WaterBuffer::with_capacity(0);
        assert_eq!(b.cap, 0);
        assert_eq!(b.len(), 0);
    }

    #[test]
    fn test_zero_capacity_operations() {
        let mut b = WaterBuffer::with_capacity(0);
        b.extend_from_slice(b"ABC");
        assert_eq!(b.len(), 0);
    }

    // ============================================================================
    // STATE CONSISTENCY
    // ============================================================================

    #[test]
    fn test_circular_position_consistency() {
        let mut b = WaterBuffer::with_capacity(5);
        assert_eq!(b.circular_position, None);

        b.extend_from_slice(b"ABCDE");
        assert_eq!(b.circular_position, None);

        b.extend_from_slice(b"F");
        assert!(b.circular_position.is_some());
    }

    #[test]
    fn test_circular_position_after_multiple_wraps() {
        let mut b = WaterBuffer::with_capacity(4);

        b.extend_from_slice(b"ABCD");
        assert_eq!(b.circular_position, None);

        b.extend_from_slice(b"EFGH");
        assert_eq!(b.circular_position, Some(0));

        b.extend_from_slice(b"IJ");
        assert_eq!(b.circular_position, Some(2));
    }

    #[test]
    fn test_start_pos_never_exceeds_capacity() {
        let mut b = WaterBuffer::with_capacity(5);
        b.extend_from_slice(b"ABCDEFGHIJKLMNOP");

        assert!(b.start_pos < b.cap);
    }

    #[test]
    fn test_len_never_exceeds_capacity() {
        let mut b = WaterBuffer::with_capacity(6);

        for _ in 0..100 {
            b.push(b'X');
            assert!(b.len() <= b.cap);
        }
    }

    // ============================================================================
    // MIXED OPERATIONS
    // ============================================================================

    #[test]
    fn test_push_advance_push_wrap() {
        let mut b = WaterBuffer::with_capacity(5);

        b.push(b'A');
        b.push(b'B');
        b.push(b'C');
        assert_eq!(&b[..], b"ABC");

        b.advance(2);
        assert_eq!(&b[..], b"C");

        b.extend_from_slice(b"DEFGH");
        assert_eq!(b.len(), 5);
    }

    #[test]
    fn test_wrap_advance_wrap_advance() {
        let mut b = WaterBuffer::with_capacity(4);

        b.extend_from_slice(b"ABCDEF"); // First wrap
        assert_eq!(b.len(), 4);

        b.advance(2); // Remove 2
        assert_eq!(b.len(), 2);

        b.extend_from_slice(b"GHIJ"); // Second wrap
        assert_eq!(b.len(), 4);

        b.advance(1);
        assert_eq!(b.len(), 3);
    }

    #[test]
    fn test_interleaved_push_and_extend() {
        let mut b = WaterBuffer::with_capacity(6);

        b.push(b'A');
        b.extend_from_slice(b"BC");
        b.push(b'D');
        b.extend_from_slice(b"EF");
        assert_eq!(&b[..], b"ABCDEF");
        b.push(b'G'); // Should wrap
        assert_eq!(b.len(), 6);
    }

    #[test]
    fn test_chunk_mut_wrap_advance() {
        let mut b = WaterBuffer::with_capacity(8);

        let chunk = b.chunk_mut();
        chunk[0..4].copy_from_slice(b"ABCD");
        b.advance_mut(4);

        b.extend_from_slice(b"EFGHIJKL"); // Should wrap
        assert_eq!(b.len(), 8);

        b.advance(4);
        assert_eq!(b.len(), 4);
    }

    // ============================================================================
    // UN_INITIALIZED_REMAINING EDGE CASES
    // ============================================================================

    #[test]
    fn test_un_initialized_remaining_progression() {
        let mut b = WaterBuffer::with_capacity(5);
        assert_eq!(b.un_initialized_remaining(), 5);

        b.push(b'A');
        assert_eq!(b.un_initialized_remaining(), 4);

        b.extend_from_slice(b"BCDE");
        assert_eq!(b.un_initialized_remaining(), 0);

        b.push(b'F'); // Wrap
        assert_eq!(b.un_initialized_remaining(), 4);
    }

    #[test]
    fn test_un_initialized_after_full_wrap() {
        let mut b = WaterBuffer::with_capacity(4);
        b.extend_from_slice(b"ABCDEFGH"); // Complete wrap

        let remaining = b.un_initialized_remaining();
        assert_eq!(remaining, 0);
    }

    // ============================================================================
    // BOUNDARY CONDITIONS
    // ============================================================================

    #[test]
    fn test_write_at_exact_boundary() {
        let mut b = WaterBuffer::with_capacity(5);
        b.extend_from_slice(b"ABCDE"); // Exactly at boundary

        assert_eq!(b.len(), 5);
        assert_eq!(b.circular_position,None);

        b.push(b'F'); // One more should trigger wrap
        assert_eq!(b.circular_position, Some(1));
    }

    #[test]
    fn test_advance_at_boundary() {
        let mut b = WaterBuffer::with_capacity(5);
        b.extend_from_slice(b"ABCDE");

        b.advance(5);
        assert_eq!(b.len(), 0);
        assert_eq!(b.start_pos, 5);
    }

    // ============================================================================
    // STRESS TESTS
    // ============================================================================

    #[test]
    fn test_many_small_writes() {
        let mut b = WaterBuffer::with_capacity(10);

        for i in 0..50 {
            b.push((i % 256) as u8);
        }

        assert_eq!(b.len(), 10);
    }

    #[test]
    fn test_alternating_write_advance() {
        let mut b = WaterBuffer::with_capacity(8);

        for _ in 0..20 {
            b.extend_from_slice(b"ABC");
            if b.len() > 3 {
                b.advance(2);
            }
        }

        assert!(b.len() <= b.cap);
    }

    #[test]
    fn test_reset_after_complex_operations() {
        let mut b = WaterBuffer::with_capacity(6);

        b.extend_from_slice(b"ABCDEF");
        b.advance(3);
        b.extend_from_slice(b"GHIJKL");
        b.push(b'X');

        b.reset();

        assert_eq!(b.len(), 0);
        assert_eq!(b.start_pos, 0);
        assert_eq!(b.circular_position, None);

        b.extend_from_slice(b"123");
        assert_eq!(&b[..], b"123");
    }

    // ============================================================================
    // INDEXING BOUNDARY TESTS
    // ============================================================================

    #[test]
    fn test_index_at_boundaries() {
        let mut b = WaterBuffer::with_capacity(5);
        b.extend_from_slice(b"ABCDE");

        assert_eq!(b[0], b'A');
        assert_eq!(b[4], b'E');
    }

    #[test]
    fn test_index_out_of_bounds() {
        let mut b = WaterBuffer::with_capacity(5);
        b.extend_from_slice(b"ABC");
         assert_eq!(b[5], b'C');
    }

    #[test]
    fn test_index_mut_boundaries() {
        let mut b = WaterBuffer::with_capacity(5);
        b.extend_from_slice(b"ABCDE");

        b[0] = b'X';
        b[4] = b'Y';

        assert_eq!(&b[..], b"XBCDY");
    }

    // ============================================================================
    // RANGE BOUNDARY TESTS
    // ============================================================================

    #[test]
    fn test_range_full_buffer() {
        let mut b = WaterBuffer::with_capacity(5);
        b.extend_from_slice(b"ABCDE");

        assert_eq!(&b[0..5], b"ABCDE");
    }

    #[test]
    fn test_range_empty() {
        let mut b = WaterBuffer::with_capacity(5);
        b.extend_from_slice(b"ABCDE");

        assert_eq!(&b[2..2], b"");
    }

    #[test]
    #[should_panic(expected = "Range out of bounds")]
    fn test_range_out_of_bounds() {
        let mut b = WaterBuffer::with_capacity(5);
        b.extend_from_slice(b"ABC");

        let _ = &b[0..5]; // Should panic
    }
}