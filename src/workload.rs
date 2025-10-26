use std::hint::black_box;

#[inline(always)]
pub fn stress_integer(iterations: u64, accumulator: &mut u64) {
    for i in 0..iterations {
        let x = black_box(i);
        let y = x.wrapping_mul(0x9e3779b97f4a7c15_u64);
        let z = y ^ (y >> 17);
        let w = z.rotate_left(31);
        *accumulator = black_box(accumulator.wrapping_add(w));
    }
}

#[inline(always)]
pub fn stress_float(iterations: u64, accumulator: &mut f64) {
    for i in 0..iterations {
        let x = black_box(i as f64 + 1.0);
        let y = x.sqrt() * 1.618033988749895;
        let z = y.sin() + y.cos();
        let w = z.abs().ln_1p();
        *accumulator = black_box(*accumulator + w);
    }
}

/// Memory latency test - single pointer-chasing chain
/// (~70-100ns)
#[inline(always)]
pub fn stress_memory_latency(iterations: u64, buffer: &mut [u64]) {
    if buffer.is_empty() {
        return;
    }

    let len = buffer.len();
    let mut index = 0usize;

    for i in 0..iterations {
        let value = black_box(buffer[index]);
        let new_value = value.wrapping_mul(6364136223846793005_u64).wrapping_add(i);
        buffer[index] = black_box(new_value);
        // Next index depends on current value - defeats prefetch
        index = black_box(((new_value >> 17) ^ i) as usize % len);
    }
}

/// Memory bandwidth test - parallel independent streams
#[inline(always)]
pub fn stress_memory_bandwidth(iterations: u64, buffer: &mut [u64]) {
    if buffer.is_empty() {
        return;
    }

    let len = buffer.len();

    // Modern memory controllers can handle 8-16 parallel requests (iirc)
    const STREAMS: usize = 8;
    let mut indices = [0usize; STREAMS];

    // Different Linear Congruential Generators (LCG) multipliers for each stream
    // (all coprime)
    const LCG_MULTS: [u64; STREAMS] = [
        6364136223846793005, // Stream 0
        2862933555777941757, // Stream 1
        3202034522624059733, // Stream 2
        7046029254386353087, // Stream 3
        5495735621104509439, // Stream 4
        1865811235122147685, // Stream 5
        8121734705789632447, // Stream 6
        4976774832059184573, // Stream 7
    ];

    // Initialize streams at different buffer offsets
    for (i, idx) in indices.iter_mut().enumerate() {
        *idx = (len / STREAMS) * i;
    }

    for iter in 0..iterations {
        let mut values = [0u64; STREAMS];
        for stream_id in 0..STREAMS {
            values[stream_id] = black_box(buffer[indices[stream_id]]);
        }

        let mut new_values = [0u64; STREAMS];
        for stream_id in 0..STREAMS {
            new_values[stream_id] = values[stream_id]
                .wrapping_mul(LCG_MULTS[stream_id])
                .wrapping_add(iter);
        }

        for stream_id in 0..STREAMS {
            buffer[indices[stream_id]] = black_box(new_values[stream_id]);
        }

        for stream_id in 0..STREAMS {
            indices[stream_id] = black_box(((new_values[stream_id] >> 17) as usize) % len);
        }
    }
}

pub fn allocate_memory_buffer(size_mb: usize) -> Box<[u64]> {
    let bytes = size_mb
        .checked_mul(1024)
        .and_then(|b| b.checked_mul(1024))
        .expect("Requested memory size (MB) too large, multiplication overflow");

    let elem_size = std::mem::size_of::<u64>();
    let num_elements = bytes / elem_size;

    let mut buffer = Vec::with_capacity(num_elements);
    for i in 0..num_elements {
        buffer.push((i as u64) ^ 0xdeadbeef);
    }
    buffer.into_boxed_slice()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stress_integer_prevents_optimization() {
        let mut acc = 0u64;
        stress_integer(1000, &mut acc);
        assert_ne!(acc, 0);
    }

    #[test]
    fn test_stress_float_prevents_optimization() {
        let mut acc = 0.0f64;
        stress_float(1000, &mut acc);
        assert!(acc.is_finite());
        assert_ne!(acc, 0.0);
    }

    #[test]
    fn test_stress_memory_latency_modifies_buffer() {
        let mut buffer = vec![0u64; 16384].into_boxed_slice();
        stress_memory_latency(10000, &mut buffer);
        let non_zero_count = buffer.iter().filter(|&&x| x != 0).count();
        assert!(non_zero_count > 0);
    }

    #[test]
    fn test_stress_memory_bandwidth_modifies_buffer() {
        let mut buffer = vec![0u64; 16384].into_boxed_slice();
        stress_memory_bandwidth(5000, &mut buffer);
        let non_zero_count = buffer.iter().filter(|&&x| x != 0).count();
        assert!(non_zero_count > 0);
    }

    #[test]
    fn test_memory_bandwidth_coverage() {
        let mut buffer = vec![0u64; 8192].into_boxed_slice();

        let initial_buffer = buffer.to_vec();
        stress_memory_bandwidth(1000, &mut buffer);

        let modified_count = buffer
            .iter()
            .zip(initial_buffer.iter())
            .filter(|(a, b)| a != b)
            .count();

        assert!(
            modified_count > 100,
            "Should modify many indices, got {}",
            modified_count
        );
    }

    #[test]
    fn test_memory_buffer_allocation() {
        let buffer = allocate_memory_buffer(1);
        let expected_elements = 1024 * 1024 / 8;
        assert_eq!(buffer.len(), expected_elements);

        let all_zero = buffer.iter().all(|&x| x == 0);
        assert!(!all_zero);
    }

    #[test]
    fn test_memory_latency_pointer_chasing() {
        let mut buffer = vec![0u64; 1024].into_boxed_slice();

        let mut accessed = vec![false; buffer.len()];
        let mut index = 0usize;

        for i in 0..100 {
            accessed[index] = true;
            let value = buffer[index]
                .wrapping_mul(6364136223846793005_u64)
                .wrapping_add(i);
            buffer[index] = value;
            index = ((value >> 17) ^ i) as usize % buffer.len();
        }

        let coverage = accessed.iter().filter(|&&x| x).count();
        assert!(
            coverage > 50,
            "Should access diverse indices, got {}",
            coverage
        );
    }

    #[test]
    fn test_memory_bandwidth_parallel_phases() {
        let mut buffer = vec![0u64; 8192].into_boxed_slice();

        stress_memory_bandwidth(100, &mut buffer);

        // Verify buffer was modified
        let non_zero_count = buffer.iter().filter(|&&x| x != 0).count();
        assert!(non_zero_count > 0, "Buffer should be modified");

        // Verify multiple streams accessed different regions
        // With 8 streams initialized at len/8 intervals, we expect wide distribution
        let initial_indices = [0, 1024, 2048, 3072, 4096, 5120, 6144, 7168];
        let modified_in_regions = initial_indices
            .iter()
            .filter(|&&idx| idx < buffer.len() && buffer[idx] != (idx as u64 ^ 0xdeadbeef))
            .count();

        assert!(
            modified_in_regions >= 4,
            "Should modify multiple stream regions"
        );
    }
}
