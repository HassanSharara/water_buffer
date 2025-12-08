#[cfg(all(test, feature = "do-not-expand"))]
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
        assert_eq!(&b[..], b"FGH");
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
        assert_eq!(b.start_pos, 2);
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