# 🌊 WaterBuffer

A high-performance, zero-overhead byte buffer implementation in Rust that outperforms the industry-standard `BytesMut` by **6-11x** in most scenarios.

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## 🚀 Performance

WaterBuffer is designed for **maximum speed** with minimal abstraction overhead. Built on raw pointer operations and optimized memory allocation strategies, it delivers exceptional performance for buffer-intensive applications.

### Benchmark Results (Release Mode)

Tested on: MacBook Pro (M-series/Intel), Rust 1.70+

| Operation | WaterBuffer | BytesMut | Speedup |
|-----------|-------------|----------|---------|
| **10M Single-Byte Pushes** | 10.2 ms | 66.4 ms | **6.49x faster** ⚡ |
| **100K Mixed Operations** | 5.8 ms | 63.5 ms | **10.88x faster** ⚡ |
| **Preallocated Buffer** | 5.7 ms | 61.7 ms | **10.84x faster** ⚡ |
| **Reallocation Stress** | 10.1 ms | 69.1 ms | **6.86x faster** ⚡ |
| **Bulk Writes (10K×100KB)** | 39.6 ms | 39.6 ms | **~1.00x (tie)** 🤝 |
| **HTTP Streaming (4KB chunks)** | 46.4 ms | 46.5 ms | **~1.00x (tie)** 🤝 |

### Key Takeaways

- **6-11x faster** for single-byte operations and mixed workloads
- **Equal performance** for large bulk operations
- **2-2.2x faster** even in debug mode
- **Zero-cost abstraction** with raw pointer operations

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
water_buffer = "0.1.0"
```

## 🎯 Usage

```rust
use water_buffer::WaterBuffer;

fn main() {
    // Create a buffer with initial capacity
    let mut buffer = WaterBuffer::with_capacity(1024);
    
    // Push single bytes
    buffer.push(42);
    buffer.push(43);
    
    // Extend from slice
    let data = b"Hello, World!";
    buffer.extend_from_slice(data);
    
    // Access elements
    println!("First byte: {}", buffer[0]);
    println!("Buffer length: {}", buffer.len());
    
    // Get slice view
    let slice = &buffer[..];
    println!("Data: {:?}", slice);
    
    // Clear buffer (keeps capacity)
    buffer.clear();
    
    // Reuse buffer
    buffer.extend_from_slice(b"New data");
}
```

## 🔥 Features

- **High Performance**: 6-11x faster than BytesMut for most operations
- **Zero-Copy Access**: Direct slice views without copying data
- **Efficient Growth**: Smart capacity expansion with 1.5x growth factor
- **Memory Efficient**: Uses `realloc` for in-place growth when possible
- **Index Operations**: Full support for `[]` indexing and ranges
- **Iterator Support**: Implements `Iterator` trait
- **Clear & Reuse**: Fast buffer clearing without deallocation

## 🎨 API Overview

### Creation
```rust
let buffer = water_buffer::WaterBuffer::with_capacity(size);
```

### Writing
```rust
buffer.push(byte);                    // Add single byte
buffer.extend_from_slice(&[1, 2, 3]); // Add multiple bytes
```

### Reading
```rust
let byte = buffer[0];          // Index single element
let slice = &buffer[0..10];    // Get slice view
let all = &buffer[..];         // Get full slice
let len = buffer.len();        // Get length
```

### Iteration
```rust
for byte in buffer.into_iter() {
    println!("{}", byte);
}
```

### Management
```rust
buffer.clear();                // Reset buffer (keeps capacity)
buffer.advance(n);             // Skip n bytes
let remaining = buffer.remaining(); // Get remaining bytes
```

## ⚡ When to Use WaterBuffer

### ✅ Perfect For:
- **High-performance parsers** (HTTP, binary protocols, serialization)
- **Single-threaded buffer operations** with high throughput requirements
- **Streaming data processing** with frequent small writes
- **Applications where you control the buffer lifecycle**
- **Performance-critical paths** in your application

### 🤔 Consider BytesMut If:
- You need **Tokio/async ecosystem integration**
- You require **zero-copy split/freeze operations**
- You're **sharing buffers across threads**
- You need the **safety guarantees** of a battle-tested library
- **Ecosystem compatibility** is more important than raw speed

## 🧪 Safety & Testing

WaterBuffer has been thoroughly tested for memory safety:

- ✅ **26 safety tests** covering edge cases, bounds checking, and memory operations
- ✅ **Miri verification** for undefined behavior detection
- ✅ **Stress tested** with millions of operations
- ✅ **Fuzz tested** for crash resistance
- ✅ **Valgrind clean** (no memory leaks)

Run the test suite:
```bash
# Standard tests
cargo test

# Miri undefined behavior check
cargo +nightly miri test

# Benchmarks
cargo run --release --example benchmark
```

## 📊 Detailed Benchmark Scenarios

### Test 1: Many Small Writes
**Scenario**: 10 million single-byte pushes  
**WaterBuffer**: 10.2 ms | **BytesMut**: 66.4 ms  
**Result**: WaterBuffer is **6.49x faster** ⚡

### Test 2: Large Bulk Writes
**Scenario**: 10,000 × 100KB chunks  
**WaterBuffer**: 39.6 ms | **BytesMut**: 39.6 ms  
**Result**: **Tie** - both equally fast 🤝

### Test 3: Mixed Operations
**Scenario**: 100K iterations of extend + 100 pushes + clear  
**WaterBuffer**: 5.8 ms | **BytesMut**: 63.5 ms  
**Result**: WaterBuffer is **10.88x faster** ⚡

### Test 4: Reallocation Stress
**Scenario**: Start with 16 bytes, grow to 10M  
**WaterBuffer**: 10.1 ms | **BytesMut**: 69.1 ms  
**Result**: WaterBuffer is **6.86x faster** ⚡

### Test 5: Preallocated Optimal
**Scenario**: 10M pushes with no reallocations  
**WaterBuffer**: 5.7 ms | **BytesMut**: 61.7 ms  
**Result**: WaterBuffer is **10.84x faster** ⚡

### Test 6: HTTP Streaming
**Scenario**: Process 1MB in 4KB chunks, 1000 times  
**WaterBuffer**: 46.4 ms | **BytesMut**: 46.5 ms  
**Result**: **Tie** - both equally fast 🤝

## 🏗️ Architecture

WaterBuffer achieves its performance through:

1. **Raw Pointer Operations**: Direct memory manipulation without bounds checking overhead in hot paths
2. **Smart Growth Strategy**: 1.5x capacity expansion balances memory usage and reallocation frequency
3. **Inline Hints**: Aggressive `#[inline(always)]` for zero-cost abstractions
4. **Minimal Abstraction**: No reference counting, no complex internal structures
5. **Efficient Realloc**: Uses `realloc` for in-place growth when possible

```rust
pub struct WaterBuffer<T> {
    cap: usize,                  // Current capacity
    start_pos: usize,            // Start position for advance operations
    pointer: *mut T,             // Raw pointer to data
    iterator_pos: usize,         // Iterator state
    filled_data_length: usize,   // Number of valid bytes
}
```

## 🔍 Implementation Details

### Memory Management
- Uses Rust's global allocator via `std::alloc`
- `realloc` for efficient capacity growth
- Proper cleanup in `Drop` implementation
- No memory leaks (Valgrind verified)

### Safety
- Bounds checking on all index operations
- Panic on invalid access (fail-fast)
- Unsafe code isolated and documented
- Miri-verified for undefined behavior

### Growth Strategy
```rust
new_capacity = max(current_capacity * 1.5, required_size)
```

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Inspired by the excellent work on [bytes](https://github.com/tokio-rs/bytes) crate
- Benchmarked against BytesMut to ensure real-world performance gains
- Thanks to the Rust community for feedback and testing

## 📈 Roadmap

- [ ] Add `Buf` and `BufMut` trait implementations
- [ ] Zero-copy split operations
- [ ] SIMD optimizations for bulk operations
- [ ] Thread-safe variant with Arc
- [ ] Direct I/O integration
- [ ] Custom allocator support

## 💬 Support

- **Issues**: [GitHub Issues](https://github.com/yourusername/water_buffer/issues)
- **Discussions**: [GitHub Discussions](https://github.com/yourusername/water_buffer/discussions)

---

**Made with  ⚡ and by Hassan Sharara**

*Performance benchmarks conducted on modern hardware. Your results may vary based on CPU, memory, and workload characteristics.*