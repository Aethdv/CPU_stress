use std::hint::black_box;

#[inline(always)]
pub fn stress_integer(iterations: u64, accumulator: &mut u64) {
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
pub fn stress_float(iterations: u64, accumulator: &mut f64) {
    for i in 0..iterations {
        let x = black_box(i as f64 + 1.0);
        let y = x.sqrt() * 1.618033988749895;
        let z = y.sin() + y.cos();
        let w = z.abs().ln_1p();
        *accumulator = black_box(*accumulator + w);
    }
}

#[inline(always)]
pub fn stress_memory(iterations: u64, buffer: &mut [u64]) {
    if buffer.is_empty() {
        return;
    }

    let len = buffer.len();
    let mut index = 0usize;

    for i in 0..iterations {
        let value = black_box(buffer[index]);
        let new_value = value.wrapping_mul(6364136223846793005_u64).wrapping_add(i);
        buffer[index] = black_box(new_value);
        index = black_box(((new_value >> 17) ^ i) as usize % len);
    }
}

pub fn allocate_memory_buffer(size_mb: usize) -> Box<[u64]> {
    let num_elements = (size_mb * 1024 * 1024) / std::mem::size_of::<u64>();

    let mut buffer = Vec::with_capacity(num_elements);
    for i in 0..num_elements {
        buffer.push(i as u64 ^ 0xDEADBEEF);
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
    fn test_stress_memory_modifies_buffer() {
        let mut buffer = vec![0u64; 16384].into_boxed_slice();
        stress_memory(10000, &mut buffer);
        let non_zero_count = buffer.iter().filter(|&&x| x != 0).count();
        assert!(non_zero_count > 0);
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
    fn test_memory_workload_pointer_chasing() {
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
}
