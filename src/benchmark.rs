use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use crate::reporting::format_number;
use crate::worker;

#[derive(Debug, Clone)]
pub struct WorkloadResult {
    pub name:        String,
    pub ops_per_sec: u64,
}

pub fn run_single_workload(
    workload: &str,
    num_threads: usize,
    memory_mb: usize,
    batch_size: u64,
    duration_secs: u64,
    quiet: bool,
) -> WorkloadResult {
    if !quiet {
        println!("\n[→] Running {} workload...", workload);
    }

    let stop_signal = Arc::new(AtomicBool::new(false));
    let work_counter = Arc::new(AtomicU64::new(0));

    let handler_stop = Arc::clone(&stop_signal);
    let _ = ctrlc::set_handler(move || {
        handler_stop.store(true, Ordering::Release);
    });

    let mut handles = Vec::with_capacity(num_threads);

    for id in 0..num_threads {
        let stop = Arc::clone(&stop_signal);
        let counter = Arc::clone(&work_counter);
        let wl = workload.to_string();
        let batch = batch_size;
        let mem_mb = memory_mb;

        let handle = thread::spawn(move || {
            worker::worker_thread(id, stop, counter, &wl, batch, mem_mb);
        });
        handles.push(handle);
    }

    let start = Instant::now();
    let duration_limit = Duration::from_secs(duration_secs);

    if !quiet {
        let report_stop = Arc::clone(&stop_signal);
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
                last_ops = current_ops;

                print!(
                    "\r  [Running] Total ops: {} | Rate: {}/s    ",
                    format_number(current_ops),
                    format_number(ops_per_sec)
                );
                std::io::stdout().flush().unwrap();
            }
        });
    }

    loop {
        thread::sleep(Duration::from_millis(100));

        if stop_signal.load(Ordering::Relaxed) {
            break;
        }

        if start.elapsed() >= duration_limit {
            stop_signal.store(true, Ordering::Release);
            break;
        }
    }

    for handle in handles {
        let _ = handle.join();
    }

    let elapsed = start.elapsed();
    let total_ops = work_counter.load(Ordering::Relaxed);
    let ops_per_sec = if elapsed.as_secs() > 0 {
        total_ops / elapsed.as_secs()
    } else {
        total_ops
    };

    if !quiet {
        println!(
            "\r  [✓] Complete: {} ops in {:.2}s               ",
            format_number(total_ops),
            elapsed.as_secs_f64()
        );
    }

    WorkloadResult {
        name: workload.to_string(),
        ops_per_sec,
    }
}

pub fn display_benchmark_table(results: &[WorkloadResult], num_threads: usize) {
    let mixed_rate = results
        .iter()
        .find(|r| r.name == "mixed")
        .map(|r| r.ops_per_sec)
        .unwrap_or(1);

    println!("\n════════════════════════════════════════════════════════════════════");
    println!("  BENCHMARK RESULTS");
    println!("════════════════════════════════════════════════════════════════════");

    let order = [
        "integer",
        "float",
        "mixed",
        "memory-latency",
        "memory-bandwidth",
    ];
    let mut sorted_results: Vec<_> = order
        .iter()
        .filter_map(|&name| results.iter().find(|r| r.name == name))
        .collect();

    for result in results {
        if !sorted_results.iter().any(|r| r.name == result.name) {
            sorted_results.push(result);
        }
    }

    println!("┌──────────────────┬─────────────┬──────────┬─────────────────┐");
    println!("│ Workload         │    Rate     │ Relative │ Per-Thread Rate │");
    println!("├──────────────────┼─────────────┼──────────┼─────────────────┤");

    for result in sorted_results {
        let rate_formatted = format_number(result.ops_per_sec);
        let rate_str = format!("{} /s", rate_formatted);

        let relative = if mixed_rate > 0 {
            result.ops_per_sec as f64 / mixed_rate as f64
        } else {
            1.0
        };
        let relative_str = format!("{:5.1}x", relative);

        let per_thread = result.ops_per_sec / num_threads.max(1) as u64;
        let per_thread_formatted = format_number(per_thread);
        let per_thread_str = format!("{} /s", per_thread_formatted);

        let workload_name = if result.name == "memory-latency" {
            "Memory-Latency".to_string()
        } else if result.name == "memory-bandwidth" {
            "Memory-Bandwidth".to_string()
        } else {
            result
                .name
                .chars()
                .next()
                .unwrap()
                .to_uppercase()
                .to_string()
                + &result.name[1..]
        };

        println!(
            "│ {:<16} │ {:>11} │ {:>8} │ {:>15} │",
            workload_name, rate_str, relative_str, per_thread_str
        );
    }

    println!("└──────────────────┴─────────────┴──────────┴─────────────────┘");
    println!("\nBaseline: Mixed = 1.0x | Threads: {}", num_threads);
}
