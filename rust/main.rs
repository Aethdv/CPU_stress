use std::thread; // for creating threads
use std::time::Duration; // for specifying time

use std::sync::{Arc, atomic::{AtomicBool, Ordering}, Barrier};
// std is standard library
// std::sync is a Rust standard library module that provides synchronization primitives for safe concurrency, like mutexes, atomic reference counting, and channels.

// Arc is an Atomically Referenced Counter, which allows for safe, shared ownership of data across the threads.
// atomic is a submodule within std::sync that provides atomic types and operations for lock-free thread-safe manipulation of primitive data (like booleans and integers).
// AtomicBool is a boolean that can be safely modified by multiple threads at once.
// Ordering specifies memory ordering constraints for atomic operations.


/// id: usize: A unique number (integer) identifying this worker thread, used for logging. (for x64-86 it's 64-bit, for x32 it's 32-bit, basically depends on a hardware)
/// stop_flag: Arc<AtomicBool>`: Shared signal watched by this worker. When set to true, the worker stops.
fn brute_force_worker(id: usize, stop_flag: Arc<AtomicBool>, barrier: Arc<Barrier>) {
    println!("✅ Starting worker thread {}:{}", std::process::id(), id);
    barrier.wait();

    loop {
        
        // Check if the stop flag is set; if yes, exit the loop and stop working.
        if stop_flag.load(Ordering::SeqCst) {
            
            break;
        }
        
        // Nested loops performing simple multiplications repeatedly.
        // This simulates CPU workload to stress test the processor.
        // from 0 up to (but not including) 2000
        // So `i` and `j` will each take values 0, 1, 2, ..., 1999.
        // This creates two nested loops, running 2000 × 2000 = 4 million iterations per full cycle.
        for i in 0..2000 {
            for j in 0..2000 {

                if j % 200 == 0 && stop_flag.load(Ordering::SeqCst) { // SeqCst (Sequentially Consistent),
                                                                      //  ensures all threads see atomic operations in the exact same order, preventing race conditions.
                    break; 
                }
                
               let _ = i * j; // intentionally ignoring the value and multiply i with j
                               // we ignore the value to not store it anywhere and uh, use it?
            }
        }
    }

    println!("🛑 Worker thread {}:{} stopping.", std::process::id(), id); // The colon is for visual formatting bs
}

fn main() {
    // Configuration
    let minutes = 1;
    let seconds = minutes * 60;

    // num_cpus::get() returns the number of cpu threads.
    let num_cores = num_cpus::get();

    println!("============================================================");
    println!("☢️ CPU STRESS TEST ☢️");
    println!("Detected {} threads. A worker will be created for each.", num_cores);
    println!("The test will run for {} minutes.", minutes);
    println!("WARNING: This is designed to push your CPU to 100% load.");
    println!("Monitor your temperatures. Press Ctrl+C to stop.");
    println!("============================================================");

    // Create the atomic boolean that all threads will share.
    // We wrap it in `Arc` to allow shared ownership from multiple threads.

    let stop_signal = Arc::new(AtomicBool::new(false)); // Arc::new - takes that atomic boolean and wraps it inside an Arc
                                                        // (a special pointer that allows many threads to share ownership safely).
                                                        
                                                        // AtomicBool::new(false) - creates a new atomic boolean set to false.



    // Ctrl+C Interrupt Handler
    // We need to handle the case where the user presses Ctrl+C.
    // We clone the Arc for the handler. This increases the reference count,
    // allowing the handler to share ownership of the stop_signal.
    let handler_stop_signal = Arc::clone(&stop_signal);
    ctrlc::set_handler(move || {
        println!("\n🛑 User interruption detected. Sending stop signal...");
        // Set the atomic boolean to true. This will signal all worker threads to exit their loops.
        handler_stop_signal.store(true, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    // A handler for Ctrl+C that sets the stop flag to true,
    // signaling workers to stop when user interrupts.
    let mut thread_handles = vec![]; // empty vector to keep track of the handles for all the threads that will start.
    let barrier = Arc::new(Barrier::new(num_cores + 1));

    for i in 0..num_cores { // e.g 16 threads, counts from 0 to 15

        let worker_stop_signal = Arc::clone(&stop_signal); // new pointer to the same shared stop flag so each thread can use it safely without copying the data.
                                                          // clone is "just copy bits blindly"
                                                          // copy is "make a new handle and update ownership info".
        let worker_barrier = Arc::clone(&barrier);

        let handle = thread::spawn(move || { // spawns new threads in parallel, each call is one new thread
                                             // || is every variable inside of the current outside active cursive bracket
                                             // move - Take ownership of the variables used from outside this closure                                                 
            brute_force_worker(i, worker_stop_signal, worker_barrier);
        });

        thread_handles.push(handle); // saving the handle so we can wait for that thread to finish
    }
    
    barrier.wait();
    println!("\n🚀 All {} workers have been started. The test is running...", num_cores);

    // This loop pauses the main thread but checks for the stop signal every second.
    // This allows for a quick exit if Ctrl+C is pressed.
    for _ in 0..seconds {
        if stop_signal.load(Ordering::SeqCst) {
            break;
        }
        thread::sleep(Duration::from_secs(1));
    }

    // This ensures the stop signal is set if the timer runs out naturally.
    if !stop_signal.load(Ordering::SeqCst) {
        println!("\n⏰ Time limit reached. Stopping all workers...");
        stop_signal.store(true, Ordering::SeqCst);
    }

    for handle in thread_handles {
        handle.join().unwrap();
    }
    
    println!("============================================================");
    println!("✅ All workers terminated. Stress test complete.");
    println!("============================================================");
}
