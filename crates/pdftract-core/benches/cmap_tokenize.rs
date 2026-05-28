//! Benchmark for CJK tokenizer performance.
//!
//! Validates that 100 KB of CJK content stream can be tokenized in < 10 ms.

use criterion::{black_box, BenchmarkId, Criterion, Throughput};
use pdftract_core::cmap::{tokenize_cjk_bytes, CodespaceRange, CodespaceRanges};

fn bench_cjk_tokenization(c: &mut Criterion) {
    let mut group = c.benchmark_group("cmap/tokenize");

    // Create a realistic CJK codespace (1-byte ASCII + 2-byte CJK)
    let mut codespace = CodespaceRanges::new();
    codespace.push(CodespaceRange::new([0x00, 0, 0, 0], [0x7F, 0, 0, 0], 1));
    codespace.push(CodespaceRange::new([0x81, 0x40, 0, 0], [0xFE, 0xFE, 0, 0], 2));

    // 10 KB of mixed ASCII/CJK content
    let mut small_input = Vec::new();
    for i in 0..5000 {
        if i % 2 == 0 {
            // ASCII
            small_input.push(0x48 + (i % 26) as u8);
        } else {
            // CJK
            small_input.push(0x81 + (i % 30) as u8);
            small_input.push(0x40 + (i % 80) as u8);
        }
    }

    // 100 KB of mixed content
    let mut large_input = Vec::new();
    for i in 0..50000 {
        if i % 2 == 0 {
            // ASCII
            large_input.push(0x48 + (i % 26) as u8);
        } else {
            // CJK
            large_input.push(0x81 + (i % 30) as u8);
            large_input.push(0x40 + (i % 80) as u8);
        }
    }

    group.throughput(Throughput::Bytes(small_input.len() as u64));
    group.bench_with_input(BenchmarkId::new("mixed", small_input.len()), &small_input, |b, input| {
        b.iter(|| {
            let mut diagnostics = Vec::new();
            black_box(tokenize_cjk_bytes(black_box(&codespace), black_box(input), &mut diagnostics));
        });
    });

    group.throughput(Throughput::Bytes(large_input.len() as u64));
    group.bench_with_input(BenchmarkId::new("mixed", large_input.len()), &large_input, |b, input| {
        b.iter(|| {
            let mut diagnostics = Vec::new();
            black_box(tokenize_cjk_bytes(black_box(&codespace), black_box(input), &mut diagnostics));
        });
    });

    group.finish();
}

fn bench_empty_codespace(c: &mut Criterion) {
    let mut group = c.benchmark_group("cmap/tokenize/empty_codespace");

    let codespace = CodespaceRanges::new();
    let mut input = vec![0x48; 100_000];

    group.throughput(Throughput::Bytes(input.len() as u64));
    group.bench_function("100KB", |b| {
        b.iter(|| {
            let mut diagnostics = Vec::new();
            black_box(tokenize_cjk_bytes(black_box(&codespace), black_box(&input), &mut diagnostics));
        });
    });

    group.finish();
}

fn bench_widest_first_matching(c: &mut Criterion) {
    let mut group = c.benchmark_group("cmap/tokenize/widest_first");

    // Create overlapping ranges to test widest-first logic
    let mut codespace = CodespaceRanges::new();
    codespace.push(CodespaceRange::new([0x00, 0, 0, 0], [0xFF, 0, 0, 0], 1));
    codespace.push(CodespaceRange::new([0x80, 0x00, 0, 0], [0xFF, 0xFF, 0, 0], 2));
    codespace.push(CodespaceRange::new([0x81, 0x40, 0x00, 0], [0xFE, 0xFE, 0xFF, 0], 3));

    // Input that will match 3-byte sequences
    let mut input = Vec::new();
    for i in 0..20000 {
        input.push(0x81 + (i % 30) as u8);
        input.push(0x40 + (i % 80) as u8);
        input.push(0x00 + (i % 50) as u8);
    }

    group.throughput(Throughput::Bytes(input.len() as u64));
    group.bench_function("3_byte_sequences", |b| {
        b.iter(|| {
            let mut diagnostics = Vec::new();
            black_box(tokenize_cjk_bytes(black_box(&codespace), black_box(&input), &mut diagnostics));
        });
    });

    group.finish();
}

criterion_group!(benches, bench_cjk_tokenization, bench_empty_codespace, bench_widest_first_matching);
criterion_main!(benches);
