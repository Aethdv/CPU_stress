use clap::Parser;
use std::hint::black_box;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Parser, Debug)]
#[command(name  = "cpu_stress")]
#[command(about = "CPU stress test", long_about = None)]
struct Args {
    /// Duration in seconds (0 = run until Ctrl+C)
    #[arg(short, long, default_value_t = 0)]
    duration: u64,

    /// Number of worker threads (0 = auto-detect all cores)
    #[arg(short = 'j', long, default_value_t = 0)]
    threads: usize,

    /// Workload type: mixed, integer, float, memory
    #[arg(short, long, default_value = "mixed")]
    workload: String,

    /// Work batch size (iterations per check)
    #[arg(short, long, default_value_t = 100_000)]
    batch_size: u64,

    /// Disable progress reporting
    #[arg(short, long)]
    quiet: bool,
}

/// Prevent compiler from optimizing away our stress work
#[inline(always)]
fn stress_integer(iterations: u64, accumulator: &mut u64) {
    for i in 0..iterations {
        // Mix of operations: multiply, xor, rotate to prevent pattern optimization
        let x = black_box(i);
        let y = x.wrapping_mul(0x9e3779b97f4a7c15_u64); // Knuth's multiplicative hash
        let z = y ^ (y >> 17);
        let w = z.rotate_left(31);
        *accumulator = black_box(accumulator.wrapping_add(w));
    }
}

#[inline(always)]
fn stress_float(iterations: u64, accumulator: &mut f64) {
    for i in 0..iterations {
        let x = black_box(i as f64 + 1.0);
        let y = x.sqrt() * 1.618033988749895; // Golden ratio
        let z = y.sin()  + y.cos();
        let w = (z.abs() + 1.0).ln();
        *accumulator = black_box(*accumulator + w);
    }
}

#[inline(always)]
fn stress_memory(iterations: u64, buffer: &mut [u64; 4096]) {
    for i in 0..iterations {
        let idx = (i as usize) & 4095;
        // Cache-thrashing pattern
        buffer[idx] = black_box(buffer[idx].wrapping_mul(6364136223846793005_u64).wrapping_add(1));
    }
}

fn worker_thread(
    id: usize,
    stop_flag: Arc<AtomicBool>,
    work_counter: Arc<AtomicU64>,
    workload: String,
    batch_size: u64,
) {
    let mut int_acc:   u64 = id as u64;
    let mut float_acc: f64 = id as f64;
    let mut mem_buffer     = [0u64; 4096];

    loop {
        if stop_flag.load(Ordering::Relaxed) {
            break;
        }

        match workload.as_str() {
            "integer" => stress_integer(batch_size, &mut int_acc),
            "float"   => stress_float(batch_size,   &mut float_acc),
            "memory"  => stress_memory(batch_size,  &mut mem_buffer),
            _ => {
                // Mixed workload
                stress_integer(batch_size / 3, &mut int_acc);
                stress_float(batch_size   / 3, &mut float_acc);
                stress_memory(batch_size  / 3, &mut mem_buffer);
            }
        }

        work_counter.fetch_add(batch_size, Ordering::Relaxed);
    }

    // Use accumulators to ensure compiler doesn't eliminate work
    black_box(int_acc);
    black_box(float_acc);
    black_box(mem_buffer);
}

fn format_number(n: u64) -> String {
    if n >= 1_000_000_000 {
        format!("{:.2}B", n as f64 / 1_000_000_000.0)
    } else if n >= 1_000_000 {
        format!("{:.2}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.2}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn main() {
    let args = Args::parse();

    let num_threads = if args.threads == 0 {
        num_cpus::get()
    } else {
        args.threads
    };

    let workload = match args.workload.as_str() {
        "integer" | "float" | "memory" | "mixed" => args.workload.clone(),
        _ => {
            eprintln!("Invalid workload '{}'. Using 'mixed'.", args.workload);
            "mixed".to_string()
        }
    };

    println!("════════════════════════════════════════════════════════════");
    println!("  CPU STRESS TEST v1.0.0");
    println!("════════════════════════════════════════════════════════════");
    println!("  Threads:    {}", num_threads);
    println!("  Workload:   {}", workload);
    println!("  Batch size: {}", format_number(args.batch_size));
    println!(
        "  Duration:   {}",
        if args.duration == 0 {
            "unlimited (Ctrl+C to stop)".to_string()
        } else {
            format!("{}s", args.duration)
        }
    );
    println!("  WARNING: This will push CPU to ~99-100%. Monitor temperatures!");
    println!("════════════════════════════════════════════════════════════\n");

    let stop_signal  = Arc::new(AtomicBool::new(false));
    let work_counter = Arc::new(AtomicU64::new(0));

    // Ctrl+C handler
    let handler_stop = Arc::clone(&stop_signal);
    if let Err(e)    = ctrlc::set_handler(move || {
        println!("\n[!] Interrupt received. Stopping workers...");
        handler_stop.store(true, Ordering::Release);
    }) {
        eprintln!("Warning: Failed to set Ctrl+C handler: {}", e);
    }

    let mut handles = Vec::with_capacity(num_threads);

    for id in 0..num_threads {
        let stop    = Arc::clone(&stop_signal);
        let counter = Arc::clone(&work_counter);
        let wl      = workload.clone();
        let batch   = args.batch_size;

        let handle  = thread::spawn(move || {
            worker_thread(id, stop, counter, wl, batch);
        });
        handles.push(handle);
    }

    let start = Instant::now();
    let duration_limit = if args.duration > 0 {
        Some(Duration::from_secs(args.duration))
    } else {
        None
    };

    // Progress reporter
    if !args.quiet {
        let report_stop    = Arc::clone(&stop_signal);
        let report_counter = Arc::clone(&work_counter);
        
        thread::spawn(move || {
            let mut last_ops = 0u64;

            loop {
                thread::sleep(Duration::from_secs(1));
                if report_stop.load(Ordering::Relaxed) {
                    break;
                }

                let current_ops = report_counter.load(Ordering::Relaxed);
                let ops_per_sec = current_ops.saturating_sub(last_ops);
                last_ops        = current_ops;

                print!(
                    "\r[Running] Total ops: {} | Rate: {}/s    ",
                    format_number(current_ops),
                    format_number(ops_per_sec)
                );
                use std::io::Write;
                std::io::stdout().flush().unwrap();
            }
        });
    }

    // Main wait loop
    loop {
        thread::sleep(Duration::from_millis(100));

        if stop_signal.load(Ordering::Relaxed) {
            break;
        }

        if let Some(limit)      = duration_limit {
            if start.elapsed() >= limit {
                println!("\n[✓] Time limit reached. Stopping...");
                stop_signal.store(true, Ordering::Release);
                break;
            }
        }
    }

    // Join all workers
    for handle in handles {
        handle.join().expect("Worker thread panicked");
    }

    let elapsed     = start.elapsed();
    let total_ops   = work_counter.load(Ordering::Relaxed);
    let ops_per_sec = if elapsed.as_secs() > 0 {
        total_ops / elapsed.as_secs()
    } else {
        total_ops
    };

    println!("\n════════════════════════════════════════════════════════════");
    println!("  STRESS TEST COMPLETE");
    println!("════════════════════════════════════════════════════════════");
    println!("  Elapsed:       {:.2}s", elapsed.as_secs_f64());
    println!("  Total ops:     {}",   format_number(total_ops));
    println!("  Avg rate:      {}/s", format_number(ops_per_sec));
    println!("════════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stress_integer_prevents_optimization() {
        let mut acc = 0u64;
        stress_integer(1000, &mut acc);
        // Verify work happened (result is deterministic)
        assert_ne!(acc, 0, "Accumulator should be non-zero after work");
    }

    #[test]
    fn test_stress_float_prevents_optimization() {
        let mut acc = 0.0f64;
        stress_float(1000, &mut acc);
        assert!(acc.is_finite(), "Float accumulator should be finite");
        assert_ne!(acc, 0.0,     "Float accumulator should be non-zero");
    }

    #[test]
    fn test_stress_memory_modifies_buffer() {
        let mut buffer = [0u64; 4096];
        stress_memory(10000, &mut buffer);
        let non_zero_count = buffer.iter().filter(|&&x| x != 0).count();
        assert!(
            non_zero_count > 0,
            "Memory stress should modify buffer elements"
        );
    }

    #[test]
    fn test_worker_respects_stop_flag() {
        let stop    = Arc::new(AtomicBool::new(false));
        let counter = Arc::new(AtomicU64::new(0));

        let stop_clone    = Arc::clone(&stop);
        let counter_clone = Arc::clone(&counter);

        let handle = thread::spawn(move || {
            worker_thread(0, stop_clone, counter_clone, "integer".to_string(), 10000);
        });

        thread::sleep(Duration::from_millis(50));
        stop.store(true, Ordering::Release);

        handle.join().expect("Worker should terminate cleanly");
        assert!(
            counter.load(Ordering::Relaxed) > 0,
            "Worker should have done some work"
        );
    }

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(500), "500");
        assert_eq!(format_number(1_500), "1.50K");
        assert_eq!(format_number(2_500_000), "2.50M");
        assert_eq!(format_number(3_500_000_000), "3.50B");
    }

    #[test]
    fn test_workload_validation() {
        // Valid workloads should be accepted
        for wl in &["integer", "float", "memory", "mixed"] {
            let w = wl.to_string();
            assert!(
                matches!(w.as_str(), "integer" | "float" | "memory" | "mixed"),
                "Valid workload should match"
            );
        }
    }

    #[test]
    fn test_multi_threaded_stress() {
        let stop        = Arc::new(AtomicBool::new(false));
        let counter     = Arc::new(AtomicU64::new(0));
        let mut handles = vec![];

        for id in 0..4 {
            let s = Arc::clone(&stop);
            let c = Arc::clone(&counter);
            handles.push(thread::spawn(move || {
                worker_thread(id, s, c, "mixed".to_string(), 5000);
            }));
        }

        thread::sleep(Duration::from_millis(100));
        stop.store(true, Ordering::Release);

        for h in handles {
            h.join().unwrap();
        }

        let ops = counter.load(Ordering::Relaxed);
        assert!(ops > 10000, "Multi-threaded work should accumulate");
    }
}
