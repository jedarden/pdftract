// Benchmark for table detection.
//
// Tests the performance of line-based and borderless table detection
// on pages with varying numbers of path segments and text positions.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use pdftract_core::table::{TableDetector, PageContext};
use pdftract_core::parser::pages::PageDict;
use std::sync::Arc;
use pdftract_core::parser::object::ObjRef;
use pdftract_core::parser::resources::ResourceDict;

fn make_page() -> PageDict {
    PageDict {
        obj_ref: ObjRef::new(1, 0),
        media_box: [0.0, 0.0, 612.0, 792.0],
        resources: Arc::new(ResourceDict::default()),
        contents: vec![],
        annots: vec![],
        actual_text: None,
        lang: None,
        aa: None,
        struct_parents: None,
        crop_box: None,
        bleed_box: None,
        trim_box: None,
        art_box: None,
        rotate: 0,
    }
}

/// Generate content with a specified number of segments.
/// Creates a grid-like pattern of horizontal and vertical lines.
fn generate_grid_content(num_horiz: usize, num_vert: usize) -> Vec<u8> {
    let mut content = Vec::new();

    let y_start = 100.0;
    let y_end = 700.0;
    let x_start = 50.0;
    let x_end = 550.0;

    // Horizontal lines
    for i in 0..num_horiz {
        let y = y_start + (i as f32 * (y_end - y_start) / (num_horiz.max(1) - 1) as f32);
        content.extend(format!("{} {} m {} {} l S ", x_start, y, x_end, y).as_bytes());
    }

    // Vertical lines
    for i in 0..num_vert {
        let x = x_start + (i as f32 * (x_end - x_start) / (num_vert.max(1) - 1) as f32);
        content.extend(format!("{} {} m {} {} l S ", x, y_start, x, y_end).as_bytes());
    }

    content
}

/// Generate content with text positions for borderless tables.
/// Creates a grid-like pattern of text at aligned positions.
fn generate_borderless_content(num_rows: usize, num_cols: usize) -> Vec<u8> {
    let mut content = Vec::new();

    let y_start = 700.0;
    let y_end = 100.0;
    let x_start = 50.0;
    let x_spacing = 100.0;

    // Start text block
    content.extend(b"BT ");

    // Generate text positions in a grid pattern
    for row in 0..num_rows {
        let y = y_start - (row as f32 * (y_start - y_end) / (num_rows.max(1) - 1) as f32);
        for col in 0..num_cols {
            let x = x_start + (col as f32 * x_spacing);
            // Move to position and show text
            content.extend(format!("{} {} Td (R{}C{}) Tj ", x, y, row, col).as_bytes());
        }
    }

    // End text block
    content.extend(b"ET");

    content
}

fn bench_table_detection(c: &mut Criterion) {
    let detector = TableDetector::new();
    let page = make_page();

    let mut group = c.benchmark_group("table_detection");

    // Test with increasing numbers of segments
    for (num_horiz, num_vert) in [(10, 10), (20, 20), (30, 30), (50, 50)] {
        let total_segments = num_horiz + num_vert;
        group.bench_with_input(
            BenchmarkId::new("grid_segments", total_segments),
            &total_segments,
            |b, _| {
                let content = generate_grid_content(num_horiz, num_vert);
                let ctx = PageContext::new(&page, &content);

                b.iter(|| {
                    black_box(detector.detect_line_based(black_box(&ctx)))
                });
            },
        );
    }

    // Test with 1000+ segments (dense table page)
    group.bench_function("dense_table_1000_segments", |b| {
        let content = generate_grid_content(500, 500);
        let ctx = PageContext::new(&page, &content);

        b.iter(|| {
            black_box(detector.detect_line_based(black_box(&ctx)))
        });
    });

    group.finish();
}

fn bench_borderless_detection(c: &mut Criterion) {
    let detector = TableDetector::new();
    let page = make_page();

    let mut group = c.benchmark_group("borderless_detection");

    // Test with increasing numbers of text positions (rows * cols)
    for (num_rows, num_cols) in [(3, 3), (5, 5), (10, 10), (20, 20), (50, 50), (70, 72)] {
        let total_positions = num_rows * num_cols;
        group.bench_with_input(
            BenchmarkId::new("text_positions", total_positions),
            &total_positions,
            |b, _| {
                let content = generate_borderless_content(num_rows, num_cols);
                let ctx = PageContext::new(&page, &content);

                b.iter(|| {
                    black_box(detector.detect_borderless(black_box(&ctx)))
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_table_detection, bench_borderless_detection);
criterion_main!(benches);
