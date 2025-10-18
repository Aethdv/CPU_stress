# CPU Stress Test v1.0.0

CPU stress testing tool with computational multi-workload types. (Targeting ~99-100% load, I recommend using btop or equivalent for monitoring your temperatures).

## Requirements

- **Rust 1.88.0+** (edition 2024)

## Features

* **Effective CPU saturation** using `black_box` to prevent dead-code elimination
* **Multiple workload types**: integer math, floating-point, memory thrashing, mixed
* **Real-time metrics**: operations/sec, total work done
* **Configurable**: threads, duration, batch size via CLI
* **Full test coverage** with integration tests
* **Benchmarks** included

## Quick Start

```bash
# Build optimized release binary
cargo build --release

# Run with auto-detected cores until Ctrl+C
./target/release/cpu_stress

# Run for 60 seconds with 8 threads, float workload
./target/release/cpu_stress --duration 60 --threads 8 --workload float

# Quiet mode (no progress output)
./target/release/cpu_stress --duration 30 --quiet
```

### Test & Bench
```bash
# Testing
cargo test --release

# Benchmark (µs is a microsecond)
cargo bench
```

## CLI options
```bash
-d, --duration <SECONDS>      Duration in seconds (0 = unlimited)  [default: 0]
-j, --threads <N>             Worker threads (0 = auto-detect)     [default: 0]
-w, --workload <TYPE>         Workload: mixed|integer|float|memory [default: mixed]
-b, --batch-size <N>          Iterations per stop-check            [default: 100000]
-q, --quiet                   Disable progress reporting
-h, --help                    Print help
```