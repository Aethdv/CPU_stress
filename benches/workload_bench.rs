use criterion::{black_box, criterion_group, criterion_main, Criterion};
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
        let z = y.sin()  + y.cos();
        let w = (z.abs() + 1.0).ln();
        *accumulator = std_black_box(*accumulator + w);
    }
}

#[inline(always)]
fn stress_memory(iterations: u64, buffer: &mut [u64; 4096]) {
    for i in 0..iterations {
        let idx     = (i as usize) & 4095;
        buffer[idx] = std_black_box(
            buffer[idx]
                .wrapping_mul(6364136223846793005_u64)
                .wrapping_add(1),
        );
    }
}

fn bench_integer_workload(c: &mut Criterion) {
    c.bench_function("stress_integer_10k", |b| {
        b.iter(|| {
            let mut acc = 0u64;
            stress_integer(black_box(10_000), &mut acc);
            acc
        });
    });
}

fn bench_float_workload(c: &mut Criterion) {
    c.bench_function("stress_float_10k", |b| {
        b.iter(|| {
            let mut acc = 0.0f64;
            stress_float(black_box(10_000), &mut acc);
            acc
        });
    });
}

fn bench_memory_workload(c: &mut Criterion) {
    c.bench_function("stress_memory_10k", |b| {
        b.iter(|| {
            let mut buffer = [0u64; 4096];
            stress_memory(black_box(10_000), &mut buffer);
            buffer
        });
    });
}

criterion_group!(
    benches,
    bench_integer_workload,
    bench_float_workload,
    bench_memory_workload
);
criterion_main!(benches);