use clap::Parser;
use std::hint::black_box;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

/// Print colorized help message
fn print_help() {
    use anstyle::{AnsiColor, Color, Style};
    
    let header = Style::new().bold().fg_color(Some(Color::Ansi(AnsiColor::Cyan)));
    let cmd = Style::new().bold().fg_color(Some(Color::Ansi(AnsiColor::Green)));
    let opt = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Green)));
    let value = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Yellow)));
    let desc = Style::new();
    let example = Style::new().fg_color(Some(Color::Ansi(AnsiColor::BrightBlack)));
    let reset = Style::new();
    
    println!("{}cpu_stress{} {}", cmd, reset, env!("CARGO_PKG_VERSION"));
    println!("CPU stress testing tool with computational multi-workload types\n");
    
    println!("{}USAGE:{}", header, reset);
    println!("    {}cpu_stress{} [OPTIONS]\n", cmd, reset);
    
    println!("{}OPTIONS:{}", header, reset);
    
    // Basic options
    println!("  {}-d{}, {}--duration{} {}SECS{}", opt, reset, opt, reset, value, reset);
    println!("      {}Duration in seconds (0 = run until Ctrl+C) [default: 0]{}", desc, reset);
    
    println!("\n  {}-j{}, {}--threads{} {}NUM{}", opt, reset, opt, reset, value, reset);
    println!("      {}Number of worker threads (0 = auto-detect all cores) [default: 0]{}", desc, reset);
    
    println!("\n  {}-w{}, {}--workload{} {}TYPE{}", opt, reset, opt, reset, value, reset);
    println!("      {}Workload type: integer, float, memory, mixed [default: mixed]{}", desc, reset);
    
    // Memory options
    println!("\n  {}-m{}, {}--memory-mb{} {}MB{}", opt, reset, opt, reset, value, reset);
    println!("      {}Memory buffer size in MB (0 = auto-detect, overrides -x) [default: 0]{}", desc, reset);
    
    println!("\n  {}-x{}, {}--memory-multiplier{} {}NUM{}", opt, reset, opt, reset, value, reset);
    println!("      {}Memory multiplier for auto-detection{}", desc, reset);
    println!("      {}2=light, 4=balanced, 8=aggressive, 16=extreme [default: 4]{}", desc, reset);
    
    // Advanced options
    println!("\n  {}-b{}, {}--batch-size{} {}NUM{}", opt, reset, opt, reset, value, reset);
    println!("      {}Work batch size (iterations between stop checks) [default: 100000]{}", desc, reset);
    
    println!("\n  {}-q{}, {}--quiet{}", opt, reset, opt, reset);
    println!("      {}Disable progress reporting{}", desc, reset);
    
    println!("\n  {}-B{}, {}--benchmark{}", opt, reset, opt, reset);
    println!("      {}Run all workloads sequentially and display comparison table{}", desc, reset);
    
    println!("\n  {}-h{}, {}--help{}", opt, reset, opt, reset);
    println!("      {}Print this help message{}", desc, reset);
    
    println!("\n  {}-V{}, {}--version{}", opt, reset, opt, reset);
    println!("      {}Print version information{}", desc, reset);
    
    // Examples section
    println!("\n{}EXAMPLES:{}", header, reset);
    println!("  {}# Default balanced stress test for 60 seconds{}", example, reset);
    println!("  {}cpu_stress{} -d 60\n", cmd, reset);
    
    println!("  {}# Aggressive memory stress (8x multiplier){}", example, reset);
    println!("  {}cpu_stress{} -w memory -d 120 -x 8\n", cmd, reset);
    
    println!("  {}# Run full benchmark suite{}", example, reset);
    println!("  {}cpu_stress{} --benchmark -d 30\n", cmd, reset);
    
    println!("  {}# Extreme stress: 16 threads, 16x memory, 5 minutes{}", example, reset);
    println!("  {}cpu_stress{} -j 16 -x 16 -d 300\n", cmd, reset);
    
    println!("  {}# Manual memory size override (512 MB per thread){}", example, reset);
    println!("  {}cpu_stress{} -w memory -m 512 -d 60", cmd, reset);
}

/// Print version information
fn print_version() {
    use anstyle::{AnsiColor, Color, Style};
    
    let cmd = Style::new().bold().fg_color(Some(Color::Ansi(AnsiColor::Green)));
    let reset = Style::new();
    
    println!("{}cpu_stress{} {}", cmd, reset, env!("CARGO_PKG_VERSION"));
}

#[derive(Parser, Debug)]
#[command(name = "cpu_stress")]
#[command(version, about = "CPU stress test with memory subsystem pressure", long_about = None)]
struct Args {
    /// Duration in seconds (0 = run until Ctrl+C)
    #[arg(short, long, default_value_t = 0)]
    duration: u64,

    /// Number of worker threads (0 = auto-detect all cores)
    #[arg(short = 'j', long, default_value_t = 0)]
    threads: usize,

    /// Workload type: integer, float, memory, mixed
    #[arg(short, long, default_value = "mixed")]
    #[arg(value_parser = ["integer", "float", "memory", "mixed"])]
    workload: String,

    /// Memory buffer size in MB (0 = auto-detect, overrides -x)
    #[arg(short = 'm', long, default_value_t = 0)]
    memory_mb: usize,

    /// Memory multiplier (2=light, 4=balanced, 8=aggressive, 16=extreme)
    #[arg(short = 'x', long, default_value_t = 4)]
    memory_multiplier: usize,

    /// Work batch size (iterations between stop checks)
    #[arg(short, long, default_value_t = 100_000)]
    batch_size: u64,

    /// Disable progress reporting
    #[arg(short, long)]
    quiet: bool,

    /// Run all workloads sequentially and display comparison table
    #[arg(short = 'B', long)]
    benchmark: bool,
}

/// Result from a single workload run
#[derive(Debug, Clone)]
struct WorkloadResult {
    name: String,
    ops_per_sec: u64,
}

/// Cross-platform L3 cache detection
fn detect_l3_cache() -> Option<usize> {
    #[cfg(target_os = "linux")]
    {
        detect_l3_cache_linux()
    }
    
    #[cfg(target_os = "windows")]
    {
        detect_l3_cache_windows()
    }
    
    #[cfg(target_os = "macos")]
    {
        detect_l3_cache_macos()
    }
    
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        None
    }
}

/// Detect L3 cache size on Linux via sysfs
#[cfg(target_os = "linux")]
fn detect_l3_cache_linux() -> Option<usize> {
    use std::fs;
    
    // Try reading L3 cache size from sysfs
    // Path: /sys/devices/system/cpu/cpu0/cache/index{N}/size
    // L3 is usually index 3, but can vary
    
    for index in 0..=10 {
        let level_path = format!("/sys/devices/system/cpu/cpu0/cache/index{}/level", index);
        let size_path = format!("/sys/devices/system/cpu/cpu0/cache/index{}/size", index);
        
        if let Ok(level) = fs::read_to_string(&level_path) {
            if level.trim() == "3" {
                if let Ok(size_str) = fs::read_to_string(&size_path) {
                    if let Some(mb) = parse_cache_size(&size_str) {
                        return Some(mb);
                    }
                }
            }
        }
    }
    
    None
}

/// Detect L3 cache size on Windows via Win32 API
#[cfg(target_os = "windows")]
fn detect_l3_cache_windows() -> Option<usize> {
    use windows_sys::Win32::System::SystemInformation::{
        GetLogicalProcessorInformationEx, RelationCache, 
        SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX,
    };
    use std::mem;
    
    unsafe {
        let mut buffer_size: u32 = 0;
        
        // First call to get required buffer size
        GetLogicalProcessorInformationEx(RelationCache, std::ptr::null_mut(), &mut buffer_size);
        
        if buffer_size == 0 {
            return None;
        }
        
        // Allocate buffer
        let mut buffer = vec![0u8; buffer_size as usize];
        let buffer_ptr = buffer.as_mut_ptr() as *mut SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX;
        
        // Second call to get actual data
        if GetLogicalProcessorInformationEx(RelationCache, buffer_ptr, &mut buffer_size) == 0 {
            return None;
        }
        
        // Parse the buffer to find L3 cache
        let mut offset = 0usize;
        while offset + mem::size_of::<SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX>() <= buffer_size as usize {
            let info = &*(buffer.as_ptr().add(offset) as *const SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX);
            
            if info.Relationship == RelationCache {
                // Access cache information
                // The structure has a union, Cache is at offset after Relationship and Size
                let cache_info_ptr = (info as *const SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX as usize 
                    + mem::size_of::<u32>()  // Relationship
                    + mem::size_of::<u32>()) // Size
                    as *const CacheDescriptor;
                
                let cache = &*cache_info_ptr;
                
                // Level 3 cache
                if cache.Level == 3 {
                    let size_mb = cache.CacheSize / (1024 * 1024);
                    if size_mb > 0 {
                        return Some(size_mb as usize);
                    }
                }
            }
            
            offset += info.Size as usize;
        }
    }
    
    None
}

// Helper struct for Windows cache descriptor
#[cfg(target_os = "windows")]
#[repr(C)]
struct CacheDescriptor {
    Level: u8,
    Associativity: u8,
    LineSize: u16,
    CacheSize: u32,
    Type: u32,
}

/// Detect L3 cache size on macOS via sysctl
#[cfg(target_os = "macos")]
fn detect_l3_cache_macos() -> Option<usize> {
    use std::process::Command;
    
    // Try sysctl hw.l3cachesize
    if let Ok(output) = Command::new("sysctl")
        .arg("-n")
        .arg("hw.l3cachesize")
        .output() 
    {
        if output.status.success() {
            if let Ok(size_str) = String::from_utf8(output.stdout) {
                if let Ok(size_bytes) = size_str.trim().parse::<usize>() {
                    if size_bytes > 0 {
                        return Some(size_bytes / (1024 * 1024));
                    }
                }
            }
        }
    }
    
    // Fallback: try parsing sysctl -a output
    if let Ok(output) = Command::new("sysctl")
        .arg("hw.cachesize")
        .output()
    {
        if output.status.success() {
            if let Ok(output_str) = String::from_utf8(output.stdout) {
                // Output format: hw.cachesize: 65536 32768 262144 0 0 0 0 0 0 0
                // Index 2 is L3 cache size in bytes
                for line in output_str.lines() {
                    if line.starts_with("hw.cachesize:") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() > 3 {
                            if let Ok(l3_bytes) = parts[3].parse::<usize>() {
                                if l3_bytes > 0 {
                                    return Some(l3_bytes / (1024 * 1024));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    None
}

/// Parse cache size string like "8192K" or "12M" into MB
fn parse_cache_size(s: &str) -> Option<usize> {
    let s = s.trim();
    
    if s.ends_with('K') || s.ends_with('k') {
        let kb: usize = s[..s.len()-1].parse().ok()?;
        Some(kb / 1024)
    } else if s.ends_with('M') || s.ends_with('m') {
        s[..s.len()-1].parse().ok()
    } else {
        // Assume bytes
        let bytes: usize = s.parse().ok()?;
        Some(bytes / (1024 * 1024))
    }
}

/// Get total system RAM in MB (cross-platform)
fn get_total_system_ram_mb() -> Option<usize> {
    #[cfg(target_os = "linux")]
    {
        use std::fs;
        // Read /proc/meminfo
        if let Ok(contents) = fs::read_to_string("/proc/meminfo") {
            for line in contents.lines() {
                if line.starts_with("MemTotal:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        if let Ok(kb) = parts[1].parse::<usize>() {
                            return Some(kb / 1024);
                        }
                    }
                }
            }
        }
    }
    
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
        use std::mem;
        
        unsafe {
            let mut mem_info: MEMORYSTATUSEX = mem::zeroed();
            mem_info.dwLength = mem::size_of::<MEMORYSTATUSEX>() as u32;
            
            if GlobalMemoryStatusEx(&mut mem_info) != 0 {
                let total_mb = (mem_info.ullTotalPhys / (1024 * 1024)) as usize;
                return Some(total_mb);
            }
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        
        if let Ok(output) = Command::new("sysctl")
            .arg("-n")
            .arg("hw.memsize")
            .output()
        {
            if output.status.success() {
                if let Ok(size_str) = String::from_utf8(output.stdout) {
                    if let Ok(size_bytes) = size_str.trim().parse::<usize>() {
                        return Some(size_bytes / (1024 * 1024));
                    }
                }
            }
        }
    }
    
    None
}

/// Auto-detect L3 cache size and return recommended memory buffer size
/// Returns size in MB (multiplier × L3 cache, minimum 32 MB, validated against system RAM)
fn detect_memory_size(multiplier: usize) -> usize {
    let num_cpus = num_cpus::get();
    
    // Try platform-specific L3 cache detection
    if let Some(l3_mb) = detect_l3_cache() {
        let recommended = (l3_mb * multiplier).max(32);
        
        // Sanity check: ensure total allocation won't exceed 80% of system RAM
        if let Some(total_ram_mb) = get_total_system_ram_mb() {
            let total_allocation_mb = recommended * num_cpus;
            let max_safe_mb = ((total_ram_mb as f64) * 0.8) as usize;
            
            if total_allocation_mb > max_safe_mb {
                let adjusted = (max_safe_mb / num_cpus).max(32);
                eprintln!("[Auto-detect] L3 cache: {} MB → Calculated {} MB buffer per thread ({}x multiplier)", 
                          l3_mb, recommended, multiplier);
                eprintln!("[Warning] Total allocation would be {} MB ({} threads × {} MB)", 
                          total_allocation_mb, num_cpus, recommended);
                eprintln!("[Warning] Exceeds 80% of system RAM ({} MB total, {} MB limit)", 
                          total_ram_mb, max_safe_mb);
                eprintln!("[Auto-detect] Reducing to {} MB per thread (total: {} MB)", 
                          adjusted, adjusted * num_cpus);
                return adjusted;
            }
        }
        
        eprintln!("[Auto-detect] L3 cache: {} MB → Using {} MB buffer per thread ({}x multiplier)", 
                  l3_mb, recommended, multiplier);
        return recommended;
    }

    // Fallback: heuristic based on CPU count (multiplier affects these too)
    let base_heuristic = match num_cpus {
        1..=2 => 32,
        3..=4 => 64,
        5..=8 => 128,
        9..=16 => 192,
        17..=32 => 256,
        33..=64 => 512,
        65..=128 => 768,
        _ => 1024,
    };
    
    // Scale heuristic by multiplier / 4 (since base is ~4x thinking)
    let scaled = ((base_heuristic as f64) * (multiplier as f64 / 4.0)) as usize;
    let heuristic_mb = scaled.max(32);
    
    eprintln!("[Auto-detect] L3 cache unknown → Using heuristic {} MB ({}x multiplier, {} CPUs)", 
              heuristic_mb, multiplier, num_cpus);
    heuristic_mb
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

/// True memory stress using pointer-chasing to prevent prefetch
/// Buffer should be >> L3 cache size to force memory subsystem pressure
#[inline(always)]
fn stress_memory(iterations: u64, buffer: &mut [u64]) {
    if buffer.is_empty() {
        return;
    }
    
    let len = buffer.len();
    let mut index = 0usize;
    
    for i in 0..iterations {
        // Pointer-chasing: next index depends on current value
        // This prevents prefetching and forces dependent loads
        let value = black_box(buffer[index]);
        
        // Update with LCG and use high bits for next index
        let new_value = value.wrapping_mul(6364136223846793005_u64).wrapping_add(i);
        buffer[index] = black_box(new_value);
        
        // Use high-entropy bits for next index (avoid modulo bias)
        // XOR with iteration counter to ensure coverage
        index = black_box(((new_value >> 17) ^ i) as usize % len);
    }
}

/// Allocate memory buffer based on MB size
/// Returns a boxed slice to avoid stack overflow
fn allocate_memory_buffer(size_mb: usize) -> Box<[u64]> {
    let num_elements = (size_mb * 1024 * 1024) / std::mem::size_of::<u64>();
    
    // Initialize with non-zero pattern to avoid CoW optimizations
    let mut buffer = Vec::with_capacity(num_elements);
    for i in 0..num_elements {
        buffer.push(i as u64 ^ 0xDEADBEEF);
    }
    buffer.into_boxed_slice()
}

fn worker_thread(
    id: usize,
    stop_flag: Arc<AtomicBool>,
    work_counter: Arc<AtomicU64>,
    workload: String,
    batch_size: u64,
    memory_mb: usize,
) {
    let mut int_acc:   u64 = id as u64;
    let mut float_acc: f64 = id as f64;
    let mut mem_buffer = allocate_memory_buffer(memory_mb);

    loop {
        if stop_flag.load(Ordering::Relaxed) {
            break;
        }

        match workload.as_str() {
            "integer" => stress_integer(batch_size, &mut int_acc),
            "float"   => stress_float(batch_size,   &mut float_acc),
            "memory"  => stress_memory(batch_size, &mut mem_buffer),
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

/// Run a single workload and return results
fn run_single_workload(
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

    let stop_signal  = Arc::new(AtomicBool::new(false));
    let work_counter = Arc::new(AtomicU64::new(0));

    // Ctrl+C handler
    let handler_stop = Arc::clone(&stop_signal);
    let _ = ctrlc::set_handler(move || {
        handler_stop.store(true, Ordering::Release);
    });

    let mut handles = Vec::with_capacity(num_threads);

    for id in 0..num_threads {
        let stop    = Arc::clone(&stop_signal);
        let counter = Arc::clone(&work_counter);
        let wl      = workload.to_string();
        let batch   = batch_size;
        let mem_mb  = memory_mb;

        let handle  = thread::spawn(move || {
            worker_thread(id, stop, counter, wl, batch, mem_mb);
        });
        handles.push(handle);
    }

    let start = Instant::now();
    let duration_limit = Duration::from_secs(duration_secs);

    // Progress reporter
    if !quiet {
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
                    "\r  [Running] Total ops: {} | Rate: {}/s    ",
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

        if start.elapsed() >= duration_limit {
            stop_signal.store(true, Ordering::Release);
            break;
        }
    }

    // Join all workers
    for handle in handles {
        let _ = handle.join();
    }

    let elapsed     = start.elapsed();
    let total_ops   = work_counter.load(Ordering::Relaxed);
    let ops_per_sec = if elapsed.as_secs() > 0 {
        total_ops / elapsed.as_secs()
    } else {
        total_ops
    };

    if !quiet {
        println!("\r  [✓] Complete: {} ops in {:.2}s               ", 
                 format_number(total_ops), elapsed.as_secs_f64());
    }

    WorkloadResult {
        name: workload.to_string(),
        ops_per_sec,
    }
}

/// Display benchmark comparison table
fn display_benchmark_table(results: &[WorkloadResult], num_threads: usize) {
    // Find mixed workload as baseline
    let mixed_rate = results.iter()
        .find(|r| r.name == "mixed")
        .map(|r| r.ops_per_sec)
        .unwrap_or(1); // Avoid division by zero

    println!("\n════════════════════════════════════════════════════════════");
    println!("  BENCHMARK RESULTS");
    println!("════════════════════════════════════════════════════════════");
    
    // Sort results: integer, float, mixed, memory
    let order = ["integer", "float", "mixed", "memory"];
    let mut sorted_results: Vec<_> = order.iter()
        .filter_map(|&name| results.iter().find(|r| r.name == name))
        .collect();
    
    // If any workload is missing, add remaining results
    for result in results {
        if !sorted_results.iter().any(|r| r.name == result.name) {
            sorted_results.push(result);
        }
    }

    // Table header
    println!("┌──────────┬─────────────┬──────────┬─────────────────┐");
    println!("│ Workload │    Rate     │ Relative │ Per-Thread Rate │");
    println!("├──────────┼─────────────┼──────────┼─────────────────┤");

    for result in sorted_results {
        // Format rate with consistent spacing
        let rate_formatted = format_number(result.ops_per_sec);
        let rate_str = format!("{} /s", rate_formatted);
        
        // Calculate relative performance
        let relative = if mixed_rate > 0 {
            result.ops_per_sec as f64 / mixed_rate as f64
        } else {
            1.0
        };
        let relative_str = format!("{:5.1}x", relative);

        // Per-thread rate
        let per_thread = result.ops_per_sec / num_threads.max(1) as u64;
        let per_thread_formatted = format_number(per_thread);
        let per_thread_str = format!("{} /s", per_thread_formatted);

        // Capitalize first letter of workload name
        let workload_name = result.name.chars().next().unwrap().to_uppercase().to_string() + &result.name[1..];

        println!(
            "│ {:<8} │ {:>11} │ {:>8} │ {:>15} │",
            workload_name,
            rate_str,
            relative_str,
            per_thread_str
        );
    }

    println!("└──────────┴─────────────┴──────────┴─────────────────┘");
    println!("\nBaseline: Mixed = 1.0x | Threads: {}", num_threads);
}

fn main() {
    // Intercept help and version flags before clap parsing
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
        detect_memory_size(args.memory_multiplier)
    } else {
        args.memory_mb
    };

    // Benchmark mode: run all workloads
    if args.benchmark {
        if args.duration == 0 {
            eprintln!("Error: --benchmark requires --duration to be set (e.g., -d 60)");
            std::process::exit(1);
        }

        println!("════════════════════════════════════════════════════════════");
        println!("  CPU STRESS BENCHMARK v1.2.0");
        println!("════════════════════════════════════════════════════════════");
        println!("  Threads:    {}", num_threads);
        
        if args.memory_mb == 0 {
            println!("  Memory buf: {} MB per thread ({}x multiplier)", memory_mb, args.memory_multiplier);
        } else {
            println!("  Memory buf: {} MB per thread (manual)", memory_mb);
        }
        
        println!("  Batch size: {}", format_number(args.batch_size));
        println!("  Duration:   {}s per workload", args.duration);
        println!("  Total time: ~{}s (4 workloads)", args.duration * 4);
        println!("════════════════════════════════════════════════════════════");

        let workloads = ["integer", "float", "mixed", "memory"];
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
        return;
    }

    // Single workload mode (original behavior)
    let workload = match args.workload.as_str() {
        "integer" | "float" | "memory" | "mixed" => args.workload.clone(),
        _ => {
            eprintln!("Invalid workload '{}'. Using 'mixed'.", args.workload);
            "mixed".to_string()
        }
    };

    println!("════════════════════════════════════════════════════════════");
    println!("  CPU STRESS TEST v1.2.0");
    println!("════════════════════════════════════════════════════════════");
    println!("  Threads:    {}", num_threads);
    println!("  Workload:   {}", workload);
    println!("  Batch size: {}", format_number(args.batch_size));
    
    if args.memory_mb == 0 {
        println!("  Memory buf: {} MB per thread ({}x multiplier)", memory_mb, args.memory_multiplier);
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
        let mem_mb  = memory_mb;

        let handle  = thread::spawn(move || {
            worker_thread(id, stop, counter, wl, batch, mem_mb);
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
    
    // Show bandwidth for memory workload
    if workload == "memory" {
        // Each op: 1 read + 1 write = 16 bytes
        let bytes_transferred = total_ops * 16;
        let gb_per_sec = (bytes_transferred as f64) / elapsed.as_secs_f64() / 1_000_000_000.0;
        println!("  Memory BW:     {:.2} GB/s", gb_per_sec);
        println!("               (estimated, 16B per op: 8B read + 8B write)");
    }
    
    println!("════════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stress_integer_prevents_optimization() {
        let mut acc = 0u64;
        stress_integer(1000, &mut acc);
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
        let mut buffer = vec![0u64; 16384].into_boxed_slice();
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
            worker_thread(0, stop_clone, counter_clone, "integer".to_string(), 10000, 1);
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
                worker_thread(id, s, c, "mixed".to_string(), 5000, 1);
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

    #[test]
    fn test_memory_buffer_allocation() {
        let buffer = allocate_memory_buffer(1);
        let expected_elements = (1 * 1024 * 1024) / 8;
        assert_eq!(buffer.len(), expected_elements);
        
        let all_zero = buffer.iter().all(|&x| x == 0);
        assert!(!all_zero, "Buffer should be initialized with non-zero pattern");
    }

    #[test]
    fn test_memory_workload_pointer_chasing() {
        let mut buffer = vec![0u64; 1024].into_boxed_slice();
        
        let mut accessed = vec![false; buffer.len()];
        let mut index = 0usize;
        
        for i in 0..100 {
            accessed[index] = true;
            let value = buffer[index].wrapping_mul(6364136223846793005_u64).wrapping_add(i);
            buffer[index] = value;
            index = ((value >> 17) ^ i) as usize % buffer.len();
        }
        
        let coverage = accessed.iter().filter(|&&x| x).count();
        assert!(coverage > 50, "Should access diverse indices, got {}", coverage);
    }

    #[test]
    fn test_parse_cache_size() {
        assert_eq!(parse_cache_size("8192K"), Some(8));
        assert_eq!(parse_cache_size("16384K"), Some(16));
        assert_eq!(parse_cache_size("12M"), Some(12));
        assert_eq!(parse_cache_size("256M"), Some(256));
        assert_eq!(parse_cache_size("8388608"), Some(8));
    }

    #[test]
    fn test_detect_memory_size_enforces_minimum() {
        // detect_memory_size() has a built-in minimum of 32 MB regardless of hardware
        let size = detect_memory_size(4);
        assert!(size >= 32, "Auto-detect should return at least 32MB");
    }

    #[test]
    fn test_cross_platform_detection_doesnt_panic() {
        let _ = detect_l3_cache();
    }

    #[test]
    fn test_get_total_system_ram() {
        // Should detect some RAM on any system
        if let Some(ram_mb) = get_total_system_ram_mb() {
            assert!(ram_mb >= 512, "Should detect at least 512 MB RAM, got {}", ram_mb);
            assert!(ram_mb <= 2_097_152, "Unlikely to have >2 TB RAM, got {} MB", ram_mb);
        }
        // If detection fails, that's OK (might be in restricted environment)
    }

    #[test]
    fn test_ram_aware_memory_size() {
        // Auto-detect should always return a reasonable value
        let size = detect_memory_size(4);
        assert!(size >= 32, "Should be at least 32 MB");
        
        // Even on high-core systems, shouldn't be absurdly large
        let num_cpus = num_cpus::get();
        let total = size * num_cpus;
        
        // Total allocation shouldn't exceed reasonable bounds
        if let Some(ram_mb) = get_total_system_ram_mb() {
            let max_reasonable = ((ram_mb as f64) * 0.9) as usize;
            assert!(
                total <= max_reasonable,
                "Total allocation {} MB should not exceed 90% of RAM ({} MB)",
                total, ram_mb
            );
        }
    }

    #[test]
    fn test_memory_multiplier_scaling() {
        // Test different multipliers
        let size_2x = detect_memory_size(2);
        let size_4x = detect_memory_size(4);
        let size_8x = detect_memory_size(8);
        
        // Should scale (unless capped by RAM)
        assert!(size_2x >= 32);
        assert!(size_4x >= size_2x);
        assert!(size_8x >= size_4x);
    }
}
