# Locus v1.4.2
CPU benchmarking tool with computational multi-workload types.<br>
(Targeting ~99–100% load. Use `btop` or equivalent to monitor temperatures.)<br>
[![Crates.io](https://img.shields.io/crates/v/locus-cli.svg)](https://crates.io/crates/locus-cli)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Requirements

- Rust 1.88.0+ (edition 2024)

## Features

- Auto-detection
  - Detects L3 cache size (Linux/Windows/MacOS)
  - Scales memory buffers based on cache and multiplier
  - RAM-aware allocation (90% safety cap to avoid OOM)

- Workloads
  - `integer`
  - `float`
  - `memory-latency`
  - `memory-bandwidth`
  - `mixed` (integer + float + memory-latency)

- Controls
  - Threads, duration, batch size
  - Memory multiplier: 2 (light), 4 (balanced), 8 (aggressive), 16 (extreme)
  - Manual memory override (per-thread MB)

- Benchmark mode
  - Runs all workloads sequentially
  - Prints a comparison table

- Correctness
  - Uses `black_box` to avoid dead-code elimination
  - Pointer-chasing defeats prefetchers
  - Tests and Criterion benchmarks included

## Quick Start

```bash
# Build optimized release binary
cargo build --release

# Run with auto-detected cores until Ctrl+C
./target/release/locus

# Run for 10 seconds with 8 threads, float workload
./target/release/locus -w float -d 10 -j 8

# Aggressive memory latency bench (pointer chasing)
./target/release/locus -w memory-latency -d 10 -x 8

# Aggressive memory bandwidth bench (parallel streams)
./target/release/locus -w memory-bandwidth -d 10 -x 8

# Force specific buffer size (overrides auto-detect and -x)
./target/release/locus -w memory-bandwidth -d 10 -m 512

# Run a benchmark across all workload types
./target/release/locus --benchmark -d 10

# Quiet mode (no progress output)
./target/release/locus -d 10 --quiet
```

### Example output of `--benchmark`:
```bash
════════════════════════════════════════════════════════════
  BENCHMARK RESULTS
════════════════════════════════════════════════════════════
┌──────────────────┬─────────────┬──────────┬─────────────────┐
│ Workload         │    Rate     │ Relative │ Per-Thread Rate │
├──────────────────┼─────────────┼──────────┼─────────────────┤
│ Integer          │   12.70B /s │    48.0x │      793.45M /s │
│ Float            │  349.29M /s │     1.3x │       21.83M /s │
│ Mixed            │  264.62M /s │     1.0x │       16.54M /s │
│ Memory-Latency   │  105.94M /s │     0.4x │        6.62M /s │
│ Memory-Bandwidth │   32.99M /s │     0.1x │        2.06M /s │
└──────────────────┴─────────────┴──────────┴─────────────────┘
```

## Test & Development
```bash
# Run tests
cargo test --all --release

# Run micro-benchmarks (measures µs per 10K iterations)
cargo bench

# Lint checks
cargo clippy --all-targets -- -D warnings -D clippy::nursery

# Format check
cargo fmt --all
```

## CLI options
```bash
BASIC OPTIONS:
  -d, --duration <SECS>        Duration in seconds (0 = unlimited)        [default: 0]
  -j, --threads <NUM>          Worker threads (0 = auto-detect)           [default: 0]
  -w, --workload <TYPE>        Workload: integer|float|memory-latency|
                               memory-bandwidth|mixed                     [default: mixed]

MEMORY OPTIONS:
  -m, --memory-mb <MB>         Buffer size in MB (0 = auto-detect)        [default: 0]
  -x, --memory-multiplier <N>  Multiplier: 2=light, 4=balanced,
                               8=aggressive, 16=extreme                   [default: 4]

ADVANCED OPTIONS:
  -b, --batch-size <NUM>       Iterations between stop checks             [default: 100000]
  -q, --quiet                  Disable progress reporting
  -B, --benchmark              Run all workloads

  -h, --help                   Print help
  -V, --version                Print version
```
# License
This project is licensed under the [MIT](https://github.com/Aethdv/CPU_stress/blob/main/LICENSE) License.
