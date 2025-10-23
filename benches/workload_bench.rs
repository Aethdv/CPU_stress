use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box as std_black_box;

#[inline(always)]
fn stress_integer(iterations: u64, accumulator: &mut u64) {
    for i in 0..iterations {
        let x = std_black_box(i);
        let y = x.wrapping_mul(0x9e3779b97f4a7c15_u64);
        let z = y ^ (y >> 17);
        let w = z.rotate_left(31);
        *accumulator = std_black_box(accumulator.wrapping_add(w));
    }
}

#[inline(always)]
fn stress_float(iterations: u64, accumulator: &mut f64) {
    for i in 0..iterations {
        let x = std_black_box(i as f64 + 1.0);
        let y = x.sqrt() * 1.618033988749895;
        let z = y.sin() + y.cos();
        let w = z.abs().ln_1p();
        *accumulator = std_black_box(*accumulator + w);
    }
}

#[inline(always)]
fn stress_memory(iterations: u64, buffer: &mut [u64]) {
    if buffer.is_empty() {
        return;
    }

    let len = buffer.len();
    let mut index = 0usize;

    for i in 0..iterations {
        let value = std_black_box(buffer[index]);
        let new_value = value.wrapping_mul(6364136223846793005_u64).wrapping_add(i);
        buffer[index] = std_black_box(new_value);
        index = std_black_box(((new_value >> 17) ^ i) as usize % len);
    }
}

fn bench_integer_workload(c: &mut Criterion) {
    c.bench_function("stress_integer_10k", |b| {
        b.iter(|| {
            let mut acc = 0u64;
            stress_integer(std_black_box(10_000), &mut acc);
            acc
        });
    });
}

fn bench_float_workload(c: &mut Criterion) {
    c.bench_function("stress_float_10k", |b| {
        b.iter(|| {
            let mut acc = 0.0f64;
            stress_float(std_black_box(10_000), &mut acc);
            acc
        });
    });
}

fn bench_memory_workload(c: &mut Criterion) {
    c.bench_function("stress_memory_10k", |b| {
        let mut buffer = vec![0u64; 128 * 1024].into_boxed_slice();

        b.iter(|| {
            stress_memory(std_black_box(10_000), &mut buffer);
        });
    });

    c.bench_function("stress_memory_small_l1", |b| {
        let mut buffer = vec![0u64; 4096].into_boxed_slice();
        b.iter(|| stress_memory(std_black_box(10_000), &mut buffer));
    });

    c.bench_function("stress_memory_large_l3", |b| {
        let mut buffer = vec![0u64; 1024 * 1024].into_boxed_slice();
        b.iter(|| stress_memory(std_black_box(10_000), &mut buffer));
    });
}

criterion_group!(
    benches,
    bench_integer_workload,
    bench_float_workload,
    bench_memory_workload
);
criterion_main!(benches);
