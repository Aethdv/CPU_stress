use crate::workload::{
    allocate_memory_buffer, stress_float, stress_integer, stress_memory_bandwidth,
    stress_memory_latency,
};
use std::hint::black_box;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

pub fn worker_thread(
    id: usize,
    stop_flag: Arc<AtomicBool>,
    work_counter: Arc<AtomicU64>,
    workload: &str,
    batch_size: u64,
    memory_mb: usize,
) {
    let mut int_acc = id as u64;
    let mut float_acc = id as f64;
    let mut mem_buffer = allocate_memory_buffer(memory_mb);

    loop {
        if stop_flag.load(Ordering::Relaxed) {
            break;
        }

        match workload {
            "integer" => stress_integer(batch_size, &mut int_acc),
            "float" => stress_float(batch_size, &mut float_acc),
            "memory" | "memory-latency" => stress_memory_latency(batch_size, &mut mem_buffer),
            "memory-bandwidth" => stress_memory_bandwidth(batch_size, &mut mem_buffer),
            _ => {
                stress_integer(batch_size / 3, &mut int_acc);
                stress_float(batch_size / 3, &mut float_acc);
                stress_memory_latency(batch_size / 3, &mut mem_buffer);
            }
        }

        work_counter.fetch_add(batch_size, Ordering::Relaxed);
    }

    black_box(int_acc);
    black_box(float_acc);
    black_box(mem_buffer);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_worker_respects_stop_flag() {
        let stop = Arc::new(AtomicBool::new(false));
        let counter = Arc::new(AtomicU64::new(0));

        let stop_clone = Arc::clone(&stop);
        let counter_clone = Arc::clone(&counter);

        let handle = thread::spawn(move || {
            worker_thread(0, stop_clone, counter_clone, "integer", 10000, 1);
        });

        thread::sleep(Duration::from_millis(50));
        stop.store(true, Ordering::Release);

        handle.join().expect("Worker should terminate cleanly");
        assert!(counter.load(Ordering::Relaxed) > 0);
    }

    #[test]
    fn test_multi_threaded_stress() {
        let stop = Arc::new(AtomicBool::new(false));
        let counter = Arc::new(AtomicU64::new(0));
        let mut handles = vec![];

        for id in 0..4 {
            let s = Arc::clone(&stop);
            let c = Arc::clone(&counter);
            handles.push(thread::spawn(move || {
                worker_thread(id, s, c, "mixed", 5000, 1);
            }));
        }

        thread::sleep(Duration::from_millis(100));
        stop.store(true, Ordering::Release);

        for h in handles {
            h.join().unwrap();
        }

        let ops = counter.load(Ordering::Relaxed);
        assert!(ops > 10000);
    }

    #[test]
    fn test_memory_bandwidth_workload() {
        let stop = Arc::new(AtomicBool::new(false));
        let counter = Arc::new(AtomicU64::new(0));

        let stop_clone = Arc::clone(&stop);
        let counter_clone = Arc::clone(&counter);

        let handle = thread::spawn(move || {
            worker_thread(0, stop_clone, counter_clone, "memory-bandwidth", 10000, 2);
        });

        thread::sleep(Duration::from_millis(50));
        stop.store(true, Ordering::Release);

        handle.join().expect("Worker should terminate cleanly");
        assert!(counter.load(Ordering::Relaxed) > 0);
    }
}
