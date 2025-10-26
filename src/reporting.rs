use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

pub fn format_number(n: u64) -> String {
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

pub fn progress_reporter(stop_signal: Arc<AtomicBool>, work_counter: Arc<AtomicU64>) {
    let mut last_ops = 0u64;

    loop {
        thread::sleep(Duration::from_secs(1));
        if stop_signal.load(Ordering::Relaxed) {
            break;
        }

        let current_ops = work_counter.load(Ordering::Relaxed);
        let ops_per_sec = current_ops.saturating_sub(last_ops);
        last_ops = current_ops;

        print!(
            "\r[Running] Total ops: {} | Rate: {}/s    ",
            format_number(current_ops),
            format_number(ops_per_sec)
        );
        if let Err(e) = std::io::stdout().flush() {
            eprintln!("Warning: failed to flush progress output: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(500), "500");
        assert_eq!(format_number(1_500), "1.50K");
        assert_eq!(format_number(2_500_000), "2.50M");
        assert_eq!(format_number(3_500_000_000), "3.50B");
    }
}
