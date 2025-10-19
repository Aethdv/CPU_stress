# CPU Stress Test v1.2.0

CPU stress testing tool with computational multi-workload types.<br>
(Targeting ***~99-100%*** load, I recommend using btop or equivalent for monitoring your temperatures).

## Requirements

- **Rust 1.88.0+** (edition 2024)

## Features

✨ **Smart Auto-Detection**
- Automatically detects L3 cache size (Linux/Windows/macOS)
- Scales memory buffers based on detected hardware
- RAM-aware allocation (80% safety cap prevents OOM)

🔥 **Flexible Stress Levels**
- Memory multiplier control (2x light → 16x extreme)
- Integer, float, memory, and mixed workloads
- Configurable threads, duration, and batch sizes

📊 **Benchmark Mode**
- Run all workloads sequentially
- Compare performance with automatic table output
- Perfect for before/after comparisons

🎯 **Correctness Guarantees**
- Uses `black_box` to prevent compiler dead-code elimination
- Pointer-chasing memory access defeats prefetchers
- Comprehensive test coverage + benchmarks

## Quick Start

```bash
# Build optimized release binary
cargo build --release

# Run with auto-detected cores until Ctrl+C
./target/release/cpu_stress

# Run for 60 seconds with 8 threads, float workload
./target/release/cpu_stress --duration 60 --threads 8 --workload float

# Aggressive memory stress (8x multiplier)
./target/release/cpu_stress -w memory -d 120 -x 8

# Run a benchmark running all workload types sequentially
./target/release/cpu_stress --benchmark -d 30

# Force specific buffer size (ignores auto-detection and -x)
./target/release/cpu_stress -w memory -m 512 -d 60

# Quiet mode (no progress output)
./target/release/cpu_stress --duration 30 --quiet
```

### Example output of `--benchmark`:
```bash

════════════════════════════════════════════════════════════
  BENCHMARK RESULTS
════════════════════════════════════════════════════════════
┌──────────┬─────────────┬──────────┬─────────────────┐
│ Workload │    Rate     │ Relative │ Per-Thread Rate │
├──────────┼─────────────┼──────────┼─────────────────┤
│ Integer  │   13.30B /s │    47.9x │      831.07M /s │
│ Float    │  428.95M /s │     1.5x │       26.81M /s │
│ Mixed    │  277.46M /s │     1.0x │       17.34M /s │
│ Memory   │  106.49M /s │     0.4x │        6.66M /s │
└──────────┴─────────────┴──────────┴─────────────────┘
```

### Test & Development
```bash
# Run tests
cargo test --release

# Run benchmarks (measures µs per 10K iterations)
cargo bench

# Lint checks
cargo clippy -- -D warnings

# Format check
cargo fmt --all -- --check
```

## CLI options
```bash
BASIC OPTIONS:
  -d, --duration <SECS>        Duration in seconds (0 = unlimited)     [default: 0]
  -j, --threads <NUM>          Worker threads (0 = auto-detect)        [default: 0]
  -w, --workload <TYPE>        Workload: integer|float|memory|mixed    [default: mixed]

MEMORY OPTIONS:
  -m, --memory-mb <MB>          Buffer size in MB (0 = auto-detect)    [default: 0]
  -x, --memory-multiplier <NUM> Multiplier: 2=light, 4=balanced,
                                            8=aggressive, 16=extreme   [default: 4]

ADVANCED OPTIONS:
  -b, --batch-size <NUM>       Iterations between stop checks          [default: 100000]
  -q, --quiet                  Disable progress reporting
  -B, --benchmark              Run all workloads and compare results

  -h, --help                   Print help
  -V, --version                Print version
```
# License
This project is licensed under the [MIT](https://github.com/Aethdv/CPU_stress/blob/main/LICENSE) License.
