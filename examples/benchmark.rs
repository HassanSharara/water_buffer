use std::time::Instant;
use bytes::{BytesMut, BufMut};
use water_buffer::WaterBuffer;




fn benchmark_scenario<F>(name: &str, mut test_fn: F) -> u128
where
    F: FnMut(),
{
    // Warmup
    test_fn();

    let start = Instant::now();
    test_fn();
    let duration = start.elapsed();
    println!("{}: {:.3} ms", name, duration.as_micros() as f64 / 1000.0);
    duration.as_micros()
}

fn main() {
    println!("=== Comprehensive Buffer Benchmark Suite (Release Mode) ===\n");

    // ============ Test 1: Many small writes ============
    println!("Test 1: Many Small Writes (10M single byte pushes)");
    println!("─────────────────────────────────────────────────");

    let water_time = benchmark_scenario("  WaterBuffer", || {
        let mut buf = WaterBuffer::with_capacity(1024);
        for _ in 0..10_000_000 {
            buf.push(42);
        }
    });

    let bytes_time = benchmark_scenario("  BytesMut   ", || {
        let mut buf = BytesMut::with_capacity(1024);
        for _ in 0..10_000_000 {
            buf.put_u8(42);
        }
    });

    let ratio = if water_time < bytes_time {
        bytes_time as f64 / water_time as f64
    } else {
        water_time as f64 / bytes_time as f64
    };
    let winner = if water_time < bytes_time { "WaterBuffer" } else { "BytesMut" };
    println!("  → {winner} is {ratio:.2}x faster\n");

    // ============ Test 2: Large bulk writes ============
    println!("Test 2: Large Bulk Writes (10000x 100KB chunks)");
    println!("─────────────────────────────────────────────────");

    let chunk = vec![0u8; 100_000];

    let water_time = benchmark_scenario("  WaterBuffer", || {
        let mut buf = WaterBuffer::with_capacity(100_000);
        for _ in 0..10_000 {
            buf.extend_from_slice(&chunk);
            buf.clear();
        }
    });

    let bytes_time = benchmark_scenario("  BytesMut   ", || {
        let mut buf = BytesMut::with_capacity(100_000);
        for _ in 0..10_000 {
            buf.extend_from_slice(&chunk);
            buf.clear();
        }
    });

    let ratio = if water_time < bytes_time {
        bytes_time as f64 / water_time as f64
    } else {
        water_time as f64 / bytes_time as f64
    };
    let winner = if water_time < bytes_time { "WaterBuffer" } else { "BytesMut" };
    println!("  → {winner} is {ratio:.2}x faster\n");

    // ============ Test 3: Mixed operations ============
    println!("Test 3: Mixed Operations (100K iterations: extend + 100 pushes + clear)");
    println!("─────────────────────────────────────────────────");

    let chunk = vec![0u8; 1000];

    let water_time = benchmark_scenario("  WaterBuffer", || {
        let mut buf = WaterBuffer::with_capacity(2000);
        for _ in 0..100_000 {
            buf.extend_from_slice(&chunk);
            for _ in 0..100 {
                buf.push(42);
            }
            buf.clear();
        }
    });

    let bytes_time = benchmark_scenario("  BytesMut   ", || {
        let mut buf = BytesMut::with_capacity(2000);
        for _ in 0..100_000 {
            buf.extend_from_slice(&chunk);
            for _ in 0..100 {
                buf.put_u8(42);
            }
            buf.clear();
        }
    });

    let ratio = if water_time < bytes_time {
        bytes_time as f64 / water_time as f64
    } else {
        water_time as f64 / bytes_time as f64
    };
    let winner = if water_time < bytes_time { "WaterBuffer" } else { "BytesMut" };
    println!("  → {winner} is {ratio:.2}x faster\n");

    // ============ Test 4: Reallocation stress ============
    println!("Test 4: Reallocation Stress (start 16 bytes, grow to 10M)");
    println!("─────────────────────────────────────────────────");

    let water_time = benchmark_scenario("  WaterBuffer", || {
        let mut buf = WaterBuffer::with_capacity(16);
        for i in 0..10_000_000 {
            buf.push((i % 256) as u8);
        }
    });

    let bytes_time = benchmark_scenario("  BytesMut   ", || {
        let mut buf = BytesMut::with_capacity(16);
        for i in 0..10_000_000 {
            buf.put_u8((i % 256) as u8);
        }
    });

    let ratio = if water_time < bytes_time {
        bytes_time as f64 / water_time as f64
    } else {
        water_time as f64 / bytes_time as f64
    };
    let winner = if water_time < bytes_time { "WaterBuffer" } else { "BytesMut" };
    println!("  → {winner} is {ratio:.2}x faster\n");

    // ============ Test 5: Preallocated optimal case ============
    println!("Test 5: Preallocated Optimal (10M pushes, no realloc)");
    println!("─────────────────────────────────────────────────");

    let water_time = benchmark_scenario("  WaterBuffer", || {
        let mut buf = WaterBuffer::with_capacity(10_000_000);
        for _ in 0..10_000_000 {
            buf.push(42);
        }
    });

    let bytes_time = benchmark_scenario("  BytesMut   ", || {
        let mut buf = BytesMut::with_capacity(10_000_000);
        for _ in 0..10_000_000 {
            buf.put_u8(42);
        }
    });

    let ratio = if water_time < bytes_time {
        bytes_time as f64 / water_time as f64
    } else {
        water_time as f64 / bytes_time as f64
    };
    let winner = if water_time < bytes_time { "WaterBuffer" } else { "BytesMut" };
    println!("  → {winner} is {ratio:.2}x faster\n");

    // ============ Test 6: Streaming scenario (HTTP-like) ============
    println!("Test 6: Streaming Scenario (read 1MB in 4KB chunks, 1000 times)");
    println!("─────────────────────────────────────────────────");

    let chunk = vec![0u8; 4096];

    let water_time = benchmark_scenario("  WaterBuffer", || {
        let mut buf = WaterBuffer::with_capacity(8192);
        for _ in 0..1000 {
            for _ in 0..256 {  // 256 * 4KB = 1MB
                buf.extend_from_slice(&chunk);
            }
            buf.clear();
        }
    });

    let bytes_time = benchmark_scenario("  BytesMut   ", || {
        let mut buf = BytesMut::with_capacity(8192);
        for _ in 0..1000 {
            for _ in 0..256 {  // 256 * 4KB = 1MB
                buf.extend_from_slice(&chunk);
            }
            buf.clear();
        }
    });

    let ratio = if water_time < bytes_time {
        bytes_time as f64 / water_time as f64
    } else {
        water_time as f64 / bytes_time as f64
    };
    let winner = if water_time < bytes_time { "WaterBuffer" } else { "BytesMut" };
    println!("  → {winner} is {ratio:.2}x faster\n");

    println!("=== Benchmark Complete ===");
    println!("\n💡 Key Insights:");
    println!("   - WaterBuffer excels at:  push operations, raw speed");
    println!("   - Use WaterBuffer when: You need maximum speed for simple buffer operations");
}