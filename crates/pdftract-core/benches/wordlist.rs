//! Benchmark for wordlist lookup performance.
//!
//! Validates that `is_english_word` lookup is < 100 ns per word.
//! This is a critical requirement from Phase 4.7 (line 1813 of the plan).

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use pdftract_core::layout::wordlist::is_english_word;

fn bench_common_words(c: &mut Criterion) {
    // Most common words (should be fastest due to frequency sorting)
    let common_words = vec![
        "the", "of", "and", "to", "a", "in", "is", "you", "that", "it",
    ];

    let mut group = c.benchmark_group("wordlist/common");

    for word in common_words {
        group.bench_with_input(BenchmarkId::from_parameter(word), &word, |b, w| {
            b.iter(|| is_english_word(black_box(w)));
        });
    }

    group.finish();
}

fn bench_medium_frequency_words(c: &mut Criterion) {
    // Medium frequency words
    let words = vec!["computer", "program", "language", "document", "extract"];

    let mut group = c.benchmark_group("wordlist/medium");

    for word in words {
        group.bench_with_input(BenchmarkId::from_parameter(word), &word, |b, w| {
            b.iter(|| is_english_word(black_box(w)));
        });
    }

    group.finish();
}

fn bench_negative_lookups(c: &mut Criterion) {
    // Words not in the wordlist (worst case for hash table lookup)
    let not_words = vec!["xyzqwerty", "abcdefg", "nonexistentword123"];

    let mut group = c.benchmark_group("wordlist/negative");

    for word in not_words {
        group.bench_with_input(BenchmarkId::from_parameter(word), &word, |b, w| {
            b.iter(|| is_english_word(black_box(w)));
        });
    }

    group.finish();
}

fn bench_mixed_lookups(c: &mut Criterion) {
    // Mix of positive and negative lookups
    let words = vec![
        "the",
        "computer",
        "xyzqwerty",
        "document",
        "of",
        "abcdefg",
        "and",
        "program",
    ];

    let mut group = c.benchmark_group("wordlist/mixed");

    group.throughput(Throughput::Elements(words.len() as u64));

    group.bench_function("batch", |b| {
        b.iter(|| {
            for word in &words {
                black_box(is_english_word(word));
            }
        });
    });

    group.finish();
}

fn bench_case_insensitive(c: &mut Criterion) {
    // Case-insensitive lookup (requires to_lowercase())
    let words = vec!["THE", "Computer", "DoCuMeNt"];

    let mut group = c.benchmark_group("wordlist/case");

    for word in words {
        group.bench_with_input(BenchmarkId::from_parameter(word), &word, |b, w| {
            b.iter(|| is_english_word(black_box(w)));
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_common_words,
    bench_medium_frequency_words,
    bench_negative_lookups,
    bench_mixed_lookups,
    bench_case_insensitive
);
criterion_main!(benches);
