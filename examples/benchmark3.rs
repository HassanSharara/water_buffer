use std::time::Instant;
use std::hint::black_box;
use bytes::{BytesMut, BufMut, Buf};
use dhat::Profiler;
use water_buffer::WaterBuffer;

const ITERATIONS: usize = 5;

struct BenchResult {
    name: String,
    water_avg: f64,
    bytes_avg: f64,
    water_min: f64,
    bytes_min: f64,
}

impl BenchResult {
    fn print(&self) {
        let ratio = if self.water_avg < self.bytes_avg {
            self.bytes_avg / self.water_avg
        } else {
            self.water_avg / self.bytes_avg
        };
        let winner = if self.water_avg < self.bytes_avg { "WaterBuffer" } else { "BytesMut" };

        println!("  WaterBuffer: avg={:.3}ms, min={:.3}ms", self.water_avg, self.water_min);
        println!("  BytesMut:    avg={:.3}ms, min={:.3}ms", self.bytes_avg, self.bytes_min);
        println!("  → {} is {:.2}x faster (by avg)\n", winner, ratio);
    }
}

fn run_benchmark<F1, F2>(name: &str, mut water_fn: F1, mut bytes_fn: F2) -> BenchResult
where
    F1: FnMut(),
    F2: FnMut(),
{
    println!("{}", name);
    println!("{}", "─".repeat(name.len()));

    // Warmup
    water_fn();
    bytes_fn();

    let mut water_times = Vec::with_capacity(ITERATIONS);
    let mut bytes_times = Vec::with_capacity(ITERATIONS);

    for _ in 0..ITERATIONS {
        let start = Instant::now();
        water_fn();
        water_times.push(start.elapsed().as_micros() as f64 / 1000.0);

        let start = Instant::now();
        bytes_fn();
        bytes_times.push(start.elapsed().as_micros() as f64 / 1000.0);
    }

    let water_avg = water_times.iter().sum::<f64>() / ITERATIONS as f64;
    let bytes_avg = bytes_times.iter().sum::<f64>() / ITERATIONS as f64;
    let water_min = water_times.iter().cloned().fold(f64::INFINITY, f64::min);
    let bytes_min = bytes_times.iter().cloned().fold(f64::INFINITY, f64::min);

    let result = BenchResult {
        name: name.to_string(),
        water_avg,
        bytes_avg,
        water_min,
        bytes_min,
    };

    result.print();
    result
}

fn main() {

    // let pr = Profiler::new_heap();
    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║  ADVANCED BUFFER BENCHMARK SUITE - PRODUCTION SCENARIOS  ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");

    let mut results = Vec::new();

    // ============ Test 1: HTTP Request Building ============
    results.push(run_benchmark(
        "Test 1: HTTP Request Building (100K requests)",
        || {
            let headers = b"Content-Type: application/json\r\nAuthorization: Bearer token123\r\n";
            let body = b"{\"user\":\"test\",\"data\":\"payload\"}";

            for _ in 0..100_000 {
                let mut buf = WaterBuffer::with_capacity(256);
                buf.extend_from_slice(b"POST /api/endpoint HTTP/1.1\r\n");
                buf.extend_from_slice(headers);
                buf.extend_from_slice(b"Content-Length: ");
                buf.extend_from_slice(body.len().to_string().as_bytes());
                buf.extend_from_slice(b"\r\n\r\n");
                buf.extend_from_slice(body);
                black_box(buf);
            }
        },
        || {
            let headers = b"Content-Type: application/json\r\nAuthorization: Bearer token123\r\n";
            let body = b"{\"user\":\"test\",\"data\":\"payload\"}";

            for _ in 0..100_000 {
                let mut buf = BytesMut::with_capacity(256);
                buf.extend_from_slice(b"POST /api/endpoint HTTP/1.1\r\n");
                buf.extend_from_slice(headers);
                buf.extend_from_slice(b"Content-Length: ");
                buf.extend_from_slice(body.len().to_string().as_bytes());
                buf.extend_from_slice(b"\r\n\r\n");
                buf.extend_from_slice(body);
                black_box(buf);
            }
        },
    ));

    // ============ Test 2: JSON Serialization Simulation ============
    results.push(run_benchmark(
        "Test 2: JSON Serialization (50K complex objects)",
        || {
            for i in 0..50_000 {
                let mut buf = WaterBuffer::with_capacity(512);
                buf.push(b'{');
                buf.extend_from_slice(b"\"id\":");
                buf.extend_from_slice(i.to_string().as_bytes());
                buf.extend_from_slice(b",\"name\":\"user_");
                buf.extend_from_slice(i.to_string().as_bytes());
                buf.extend_from_slice(b"\",\"active\":true,\"tags\":[\"a\",\"b\",\"c\"],\"metadata\":{\"key\":\"value\"}}");
                black_box(buf);
            }
        },
        || {
            for i in 0..50_000 {
                let mut buf = BytesMut::with_capacity(512);
                buf.put_u8(b'{');
                buf.extend_from_slice(b"\"id\":");
                buf.extend_from_slice(i.to_string().as_bytes());
                buf.extend_from_slice(b",\"name\":\"user_");
                buf.extend_from_slice(i.to_string().as_bytes());
                buf.extend_from_slice(b"\",\"active\":true,\"tags\":[\"a\",\"b\",\"c\"],\"metadata\":{\"key\":\"value\"}}");
                black_box(buf);
            }
        },
    ));

    // ============ Test 3: Packet Fragmentation/Reassembly ============
    results.push(run_benchmark(
        "Test 3: Network Packet Reassembly (10K packets, 1KB each)",
        || {
            // Pre-allocate with better estimate
            let mut accumulated = WaterBuffer::with_capacity(100_000);
            let packet = vec![0u8; 1024];

            for _ in 0..10_000 {
                accumulated.extend_from_slice(&packet);
                if accumulated.len() >= 100_000 {
                    accumulated.clear();
                }
            }
            black_box(accumulated);
        },
        || {
            let mut accumulated = BytesMut::with_capacity(100_000);
            let packet = vec![0u8; 1024];

            for _ in 0..10_000 {
                accumulated.extend_from_slice(&packet);
                if accumulated.len() >= 100_000 {
                    accumulated.clear();
                }
            }
            black_box(accumulated);
        },
    ));

    // ============ Test 4: Pathological Growth (Fibonacci-like) ============
    results.push(run_benchmark(
        "Test 4: Pathological Growth Pattern (exponential reallocs)",
        || {
            let mut buf = WaterBuffer::with_capacity(8);
            for size in [1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768, 65536] {
                let chunk = vec![0u8; size];
                for _ in 0..1000 {
                    buf.extend_from_slice(&chunk);
                }
                buf.clear();
            }
            black_box(buf);
        },
        || {
            let mut buf = BytesMut::with_capacity(8);
            for size in [1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768, 65536] {
                let chunk = vec![0u8; size];
                for _ in 0..1000 {
                    buf.extend_from_slice(&chunk);
                }
                buf.clear();
            }
            black_box(buf);
        },
    ));

    // ============ Test 5: Database Row Serialization ============
    results.push(run_benchmark(
        "Test 5: Database Row Serialization (100K rows)",
        || {
            for i in 0..100_000 {
                let mut buf = WaterBuffer::with_capacity(128);
                // Simulate binary protocol: length prefix + data
                buf.push(5); // field count
                // buf.extend_from_slice(&i.to_le_bytes());
                buf.extend_from_slice(b"username_");
                buf.extend_from_slice(i.to_string().as_bytes());
                buf.push(0); // null terminator
                buf.extend_from_slice(&(i as f64 * 1.5).to_le_bytes());
                buf.push(1); // boolean
                black_box(buf);
            }
        },
        || {
            for i in 0..100_000 {
                let mut buf = BytesMut::with_capacity(128);
                buf.put_u8(5);
                // buf.extend_from_slice(&i.to_le_bytes());
                buf.extend_from_slice(b"username_");
                buf.extend_from_slice(i.to_string().as_bytes());
                buf.put_u8(0);
                buf.extend_from_slice(&(i as f64 * 1.5).to_le_bytes());
                buf.put_u8(1);
                black_box(buf);
            }
        },
    ));

    // ============ Test 6: WebSocket Frame Building ============
    results.push(run_benchmark(
        "Test 6: WebSocket Frame Encoding (50K messages)",
        || {
            let payload = b"Hello, WebSocket! This is a test message with some data.";
            for _ in 0..50_000 {
                let mut buf = WaterBuffer::with_capacity(128);
                buf.push(0x81); // FIN + text frame
                buf.push(payload.len() as u8);
                buf.extend_from_slice(payload);
                black_box(buf);
            }
        },
        || {
            let payload = b"Hello, WebSocket! This is a test message with some data.";
            for _ in 0..50_000 {
                let mut buf = BytesMut::with_capacity(128);
                buf.put_u8(0x81);
                buf.put_u8(payload.len() as u8);
                buf.extend_from_slice(payload);
                black_box(buf);
            }
        },
    ));

    // ============ Test 7: Memory Churn (Allocate/Drop) ============
    results.push(run_benchmark(
        "Test 7: Memory Churn - Allocate & Drop (100K cycles)",
        || {
            let data = vec![42u8; 1024];
            for _ in 0..100_000 {
                let mut buf = WaterBuffer::with_capacity(2048);
                buf.extend_from_slice(&data);
                buf.extend_from_slice(&data);
                black_box(&buf);
                // buf drops here
            }
        },
        || {
            let data = vec![42u8; 1024];
            for _ in 0..100_000 {
                let mut buf = BytesMut::with_capacity(2048);
                buf.extend_from_slice(&data);
                buf.extend_from_slice(&data);
                black_box(&buf);
                // buf drops here
            }
        },
    ));

    // ============ Test 8: CSV Generation ============
    results.push(run_benchmark(
        "Test 8: CSV Row Generation (50K rows, 10 columns)",
        || {
            for i in 0..50_000 {
                let mut buf = WaterBuffer::with_capacity(256);
                for col in 0..10 {
                    if col > 0 {
                        buf.push(b',');
                    }
                    buf.extend_from_slice(format!("col{}_{}", col, i).as_bytes());
                }
                buf.push(b'\n');
                black_box(buf);
            }
        },
        || {
            for i in 0..50_000 {
                let mut buf = BytesMut::with_capacity(256);
                for col in 0..10 {
                    if col > 0 {
                        buf.put_u8(b',');
                    }
                    buf.extend_from_slice(format!("col{}_{}", col, i).as_bytes());
                }
                buf.put_u8(b'\n');
                black_box(buf);
            }
        },
    ));

    // ============ Test 9: Protocol Buffer-like Encoding ============
    results.push(run_benchmark(
        "Test 9: Protobuf-style Encoding (100K messages)",
        || {
            for i in 0..100_000 {
                let mut buf = WaterBuffer::with_capacity(64);
                // Field 1: varint
                buf.push(0x08);
                buf.extend_from_slice(&encode_varint(i as u64));
                // Field 2: string
                buf.push(0x12);
                let s = format!("msg_{}", i);
                buf.push(s.len() as u8);
                buf.extend_from_slice(s.as_bytes());
                black_box(buf);
            }
        },
        || {
            for i in 0..100_000 {
                let mut buf = BytesMut::with_capacity(64);
                buf.put_u8(0x08);
                buf.extend_from_slice(&encode_varint(i as u64));
                buf.put_u8(0x12);
                let s = format!("msg_{}", i);
                buf.put_u8(s.len() as u8);
                buf.extend_from_slice(s.as_bytes());
                black_box(buf);
            }
        },
    ));

    // ============ Test 10: Extreme Reallocation Torture ============
    results.push(run_benchmark(
        "Test 10: Reallocation Torture (start 1 byte → 10MB)",
        || {
            let mut buf = WaterBuffer::with_capacity(1);
            for i in 0..10_000_000 {
                buf.push((i & 0xFF) as u8);
            }
            black_box(buf);
        },
        || {
            let mut buf = BytesMut::with_capacity(1);
            for i in 0..10_000_000 {
                buf.put_u8((i & 0xFF) as u8);
            }
            black_box(buf);
        },
    ));

    // ============ Test 11: Alternating Read/Write ============
    results.push(run_benchmark(
        "Test 11: Alternating Operations (10K cycles: write → read → clear)",
        || {
            let data = vec![42u8; 1000];
            for _ in 0..10_000 {
                let mut buf = WaterBuffer::with_capacity(2000);
                buf.extend_from_slice(&data);
                buf.extend_from_slice(&data);
                let _len = buf.len();
                let _slice = buf.as_ref();
                buf.clear();
            }
        },
        || {
            let data = vec![42u8; 1000];
            for _ in 0..10_000 {
                let mut buf = BytesMut::with_capacity(2000);
                buf.extend_from_slice(&data);
                buf.extend_from_slice(&data);
                let _len = buf.len();
                let _slice = buf.as_ref();
                buf.clear();
            }
        },
    ));

    // ============ Test 12: Log Line Formatting ============
    results.push(run_benchmark(
        "Test 12: Log Line Formatting (100K entries)",
        || {
            for i in 0..100_000 {
                let mut buf = WaterBuffer::with_capacity(256);
                buf.extend_from_slice(b"[2024-12-12 10:30:45] [INFO] ");
                buf.extend_from_slice(format!("Request {} processed successfully", i).as_bytes());
                buf.extend_from_slice(b" - duration: ");
                buf.extend_from_slice(format!("{}ms", i % 1000).as_bytes());
                buf.push(b'\n');
                black_box(buf);
            }
        },
        || {
            for i in 0..100_000 {
                let mut buf = BytesMut::with_capacity(256);
                buf.extend_from_slice(b"[2024-12-12 10:30:45] [INFO] ");
                buf.extend_from_slice(format!("Request {} processed successfully", i).as_bytes());
                buf.extend_from_slice(b" - duration: ");
                buf.extend_from_slice(format!("{}ms", i % 1000).as_bytes());
                buf.put_u8(b'\n');
                black_box(buf);
            }
        },
    ));

    // Summary
    println!("\n╔═══════════════════════════════════════════════════════════╗");
    println!("║                    BENCHMARK SUMMARY                      ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");

    let mut water_wins = 0;
    let mut bytes_wins = 0;

    for result in &results {
        if result.water_avg < result.bytes_avg {
            water_wins += 1;
            println!("✓ WaterBuffer wins: {}", result.name);
        } else {
            bytes_wins += 1;
            println!("✓ BytesMut wins:    {}", result.name);
        }
    }

    println!("\n╔═══════════════════════════════════════════════════════════╗");
    println!("║  Final Score: WaterBuffer {} - {} BytesMut", water_wins, bytes_wins);
    println!("╚═══════════════════════════════════════════════════════════╝");
}

fn encode_varint(mut value: u64) -> Vec<u8> {
    let mut buf = Vec::new();
    while value >= 0x80 {
        buf.push((value as u8) | 0x80);
        value >>= 7;
    }
    buf.push(value as u8);
    buf
}