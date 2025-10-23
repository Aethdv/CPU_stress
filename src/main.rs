mod benchmark;
mod cli;
mod reporting;
mod system;
mod worker;
mod workload;

use benchmark::{display_benchmark_table, run_single_workload};
use clap::Parser;
use cli::{Args, print_help, print_version};
use reporting::format_number;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

fn main() {
    let args_vec: Vec<String> = std::env::args().collect();

    if args_vec.len() > 1 {
        match args_vec[1].as_str() {
            "--help" | "-h" => {
                print_help();
                return;
            }
            "--version" | "-V" => {
                print_version();
                return;
            }
            _ => {}
        }
    }

    let args = Args::parse();

    let num_threads = if args.threads == 0 {
        num_cpus::get()
    } else {
        args.threads
    };

    let memory_mb = if args.memory_mb == 0 {
        system::detect_memory_size(args.memory_multiplier)
    } else {
        args.memory_mb
    };

    if args.benchmark {
        run_benchmark_mode(&args, num_threads, memory_mb);
    } else {
        run_single_mode(&args, num_threads, memory_mb);
    }
}

fn run_benchmark_mode(args: &Args, num_threads: usize, memory_mb: usize) {
    if args.duration == 0 {
        eprintln!("Error: --benchmark requires --duration to be set (e.g., -d 60)");
        std::process::exit(1);
    }

    println!("════════════════════════════════════════════════════════════");
    println!("  CPU STRESS BENCHMARK v{}", env!("CARGO_PKG_VERSION"));
    println!("════════════════════════════════════════════════════════════");
    println!("  Threads:    {}", num_threads);

    if args.memory_mb == 0 {
        println!(
            "  Memory buf: {} MB per thread ({}x multiplier)",
            memory_mb, args.memory_multiplier
        );
    } else {
        println!("  Memory buf: {} MB per thread (manual)", memory_mb);
    }

    println!("  Batch size: {}", format_number(args.batch_size));
    println!("  Duration:   {}s per workload", args.duration);
    println!("  Total time: ~{}s (5 workloads)", args.duration * 5);
    println!("════════════════════════════════════════════════════════════");

    let workloads = [
        "integer",
        "float",
        "mixed",
        "memory-latency",
        "memory-bandwidth",
    ];
    let mut results = Vec::new();

    for workload in &workloads {
        let result = run_single_workload(
            workload,
            num_threads,
            memory_mb,
            args.batch_size,
            args.duration,
            args.quiet,
        );
        results.push(result);
    }

    display_benchmark_table(&results, num_threads);
}

fn run_single_mode(args: &Args, num_threads: usize, memory_mb: usize) {
    let workload = match args.workload.as_str() {
        "integer" | "float" | "memory" | "memory-latency" | "memory-bandwidth" | "mixed" => {
            &args.workload
        }
        _ => {
            eprintln!("Invalid workload '{}'. Using 'mixed'.", args.workload);
            "mixed"
        }
    };

    println!("════════════════════════════════════════════════════════════");
    println!("  CPU STRESS TEST v{}", env!("CARGO_PKG_VERSION"));
    println!("════════════════════════════════════════════════════════════");
    println!("  Threads:    {}", num_threads);
    println!("  Workload:   {}", workload);
    println!("  Batch size: {}", format_number(args.batch_size));

    if args.memory_mb == 0 {
        println!(
            "  Memory buf: {} MB per thread ({}x multiplier)",
            memory_mb, args.memory_multiplier
        );
    } else {
        println!("  Memory buf: {} MB per thread (manual)", memory_mb);
    }

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

    let stop_signal = Arc::new(AtomicBool::new(false));
    let work_counter = Arc::new(AtomicU64::new(0));

    let handler_stop = Arc::clone(&stop_signal);
    if let Err(e) = ctrlc::set_handler(move || {
        println!("\n[!] Interrupt received. Stopping workers...");
        handler_stop.store(true, Ordering::Release);
    }) {
        eprintln!("Warning: Failed to set Ctrl+C handler: {}", e);
    }

    let mut handles = Vec::with_capacity(num_threads);

    for id in 0..num_threads {
        let stop = Arc::clone(&stop_signal);
        let counter = Arc::clone(&work_counter);
        let batch = args.batch_size;
        let mem_mb = memory_mb;
        let wl = workload.to_string();

        let handle = thread::spawn(move || {
            worker::worker_thread(id, stop, counter, &wl, batch, mem_mb);
        });
        handles.push(handle);
    }

    let start = Instant::now();
    let duration_limit = if args.duration > 0 {
        Some(Duration::from_secs(args.duration))
    } else {
        None
    };

    if !args.quiet {
        let report_stop = Arc::clone(&stop_signal);
        let report_counter = Arc::clone(&work_counter);

        thread::spawn(move || {
            reporting::progress_reporter(report_stop, report_counter);
        });
    }

    loop {
        thread::sleep(Duration::from_millis(100));

        if stop_signal.load(Ordering::Relaxed) {
            break;
        }

        if let Some(limit) = duration_limit
            && start.elapsed() >= limit
        {
            println!("\n[✓] Time limit reached. Stopping...");
            stop_signal.store(true, Ordering::Release);
            break;
        }
    }

    for handle in handles {
        handle.join().expect("Worker thread panicked");
    }

    print_final_stats(
        start.elapsed(),
        work_counter.load(Ordering::Relaxed),
        workload,
    );
}

fn print_final_stats(elapsed: Duration, total_ops: u64, workload: &str) {
    let ops_per_sec = if elapsed.as_secs() > 0 {
        total_ops / elapsed.as_secs()
    } else {
        total_ops
    };

    println!("\n════════════════════════════════════════════════════════════");
    println!("  STRESS TEST COMPLETE");
    println!("════════════════════════════════════════════════════════════");
    println!("  Elapsed:       {:.2}s", elapsed.as_secs_f64());
    println!("  Total ops:     {}", format_number(total_ops));
    println!("  Avg rate:      {}/s", format_number(ops_per_sec));

    if workload.starts_with("memory") {
        let bytes_per_op = if workload == "memory-bandwidth" {
            // Bandwidth: 8 streams × (1 read + 1 write) × 8 bytes
            128
        } else {
            // Latency: 1 read + 1 write × 8 bytes
            16
        };

        let bytes_transferred = total_ops * bytes_per_op;
        let gb_per_sec = (bytes_transferred as f64) / elapsed.as_secs_f64() / 1_000_000_000.0;
        println!("  Memory BW:     {:.2} GB/s", gb_per_sec);
        println!("               (estimated, {}B per op)", bytes_per_op);
    }

    println!("════════════════════════════════════════════════════════════");
}
