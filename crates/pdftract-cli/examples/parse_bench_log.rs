//! Parse the captured grep-corpus benchmark log into a raw-metrics JSON artifact.
//!
//! Bead bf-50mfjl. Applies the extraction/storage plumbing built for the
//! grep_1000 benchmark (bf-5b8mk: `extract_raw_timing_metrics`, bf-4b7pm:
//! `RawTimingMetrics`) to the real captured output (bf-25ewsw:
//! `notes/bf-6bvq1-benchmark.log`) and serializes the result to
//! `notes/bf-6bvq1-metrics.json`.
//!
//! The log is the verbatim capture of three runs of
//! `pdftract grep 'the' tests/fixtures/grep-corpus/corpus/`:
//! - run 1 under a pseudo-TTY (carries the tool's TTY-gated `Searched:` summary)
//! - run 2 as a plain pipe (stdout+stderr merged)
//! - run 3 with `--progress-json` (per-file `file_start`/`file_done` events
//!   on stderr, match lines on stdout)
//!
//! Run 3 is the machine-readable source of truth for file counts and matches.
//! The run 1 summary is the tool's own throughput measurement, from which the
//! wall-clock runtime is derived — the capture recorded the displayed duration
//! only to the second ("in 0.0s"), so no per-run millisecond wall time exists
//! in the log itself.
//!
//! Usage: `cargo run -p pdftract-cli --example parse_bench_log [<log>] [<out>]`

#[path = "../benches/grep_1000.rs"]
mod bench_impl;

use anyhow::{bail, ensure, Context, Result};
use bench_impl::{extract_raw_timing_metrics, RawTimingMetrics};
use serde_json::json;
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

const RUN3_EVENTS_MARKER: &str = "--- run 3 (appendix):";
const RUN3_STDOUT_MARKER: &str = "--- run 3 stdout match lines ---";
const RUN_END_PREFIX: &str = "[run ";
const MATCH_LINE_PREFIX: &str = "tests/fixtures/grep-corpus/corpus/";

/// Per-file progress-event totals parsed from a `--progress-json` section.
#[derive(Debug, Default)]
struct EventTotals {
    files_started: usize,
    files_done: usize,
    matched_files: usize,
    total_matches: usize,
    size_hint_sum: u64,
}

/// Sum the `file_start` / `file_done` progress events in a log section.
///
/// Mirrors the tolerant line-by-line JSON parsing of
/// `extract_raw_timing_metrics`: non-JSON lines are skipped, and events
/// missing a field contribute nothing for that field.
fn parse_progress_events(section: &str) -> EventTotals {
    let mut totals = EventTotals::default();
    for line in section.lines() {
        let line = line.trim();
        if line.is_empty() || !line.starts_with('{') {
            continue;
        }
        let event: serde_json::Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(_) => continue,
        };
        match event.get("type").and_then(|value| value.as_str()) {
            Some("file_start") => {
                totals.files_started += 1;
                totals.size_hint_sum += event
                    .get("size_hint")
                    .and_then(|value| value.as_u64())
                    .unwrap_or(0);
            }
            Some("file_done") => {
                totals.files_done += 1;
                let matches = event
                    .get("matches")
                    .and_then(|value| value.as_u64())
                    .unwrap_or(0) as usize;
                totals.total_matches += matches;
                if matches > 0 {
                    totals.matched_files += 1;
                }
            }
            _ => {}
        }
    }
    totals
}

/// Count match lines and distinct matched files in a stdout section.
///
/// A match line is `tests/fixtures/grep-corpus/corpus/<file>.pdf:pN:[bbox]:the`;
/// the file path ends at the first `:` (no colons in the corpus filenames).
fn count_match_lines(section: &str) -> (usize, usize) {
    let mut lines = 0usize;
    let mut files = BTreeSet::new();
    for line in section.lines() {
        if let Some(rest) = line.strip_prefix(MATCH_LINE_PREFIX) {
            let file = rest.split(':').next().unwrap_or(rest);
            if file.ends_with(".pdf") {
                lines += 1;
                files.insert(file.to_string());
            }
        }
    }
    (lines, files.len())
}

/// Parse the tool's TTY summary: `1000 files (6.6 MB) in 0.0s (133.7 MB/s)`.
///
/// The unit labels say MB/MB/s but the tool computes them in MiB
/// (bytes_total / 1024 / 1024 — see grep/progress.rs finish()).
/// Returns the last occurrence in the log (runs may be retried).
fn parse_searched_summary(log: &str) -> Result<(usize, f64, f64, f64, String)> {
    let mut last = None;
    for line in log.lines() {
        if let Some(pos) = line.find("Searched: ") {
            let tail = line[pos + "Searched: ".len()..]
                .trim()
                .trim_end_matches('\r')
                .to_string();
            last = Some(tail);
        }
    }
    let tail = match last {
        Some(tail) => tail,
        None => bail!("no 'Searched:' summary line found in log (run 1 TTY section)"),
    };

    // "1000 files (6.6 MB) in 0.0s (133.7 MB/s)"
    let parts: Vec<&str> = tail.split_whitespace().collect();
    ensure!(
        parts.len() == 8 && parts[1] == "files" && parts[4] == "in",
        "unrecognized 'Searched:' summary format: {:?}",
        tail
    );
    let files: usize = parts[0]
        .parse()
        .with_context(|| format!("parsing file count from {:?}", tail))?;
    let total_mib: f64 = parts[2]
        .trim_start_matches('(')
        .parse()
        .with_context(|| format!("parsing total from {:?}", tail))?;
    let displayed_secs: f64 = parts[5]
        .trim_end_matches('s')
        .parse()
        .with_context(|| format!("parsing duration from {:?}", tail))?;
    let mib_per_sec: f64 = parts[6]
        .trim_start_matches('(')
        .parse()
        .with_context(|| format!("parsing throughput from {:?}", tail))?;
    Ok((files, total_mib, displayed_secs, mib_per_sec, tail))
}

/// Pull `key: value` fields out of the capture header (e.g. `commit: <sha>`).
fn header_field<'a>(log: &'a str, key: &str) -> Option<&'a str> {
    log.lines().find_map(|line| {
        let rest = line.strip_prefix(key)?;
        let rest = rest.strip_prefix(':').unwrap_or(rest);
        rest.trim().split_whitespace().next()
    })
}

/// Recover the benchmarked command from the run-1 log marker line.
fn run1_command(log: &str) -> Option<String> {
    let line = log
        .lines()
        .find(|line| line.starts_with("--- run 1:"))?
        .strip_prefix("--- run 1:")?
        .trim();
    let command = match line.find("  (") {
        Some(end) => &line[..end],
        None => line,
    };
    Some(command.trim().to_string())
}

/// Slice a run section `(start_after_marker, end_before_marker)` from the log.
fn run_bounds(log: &str, start_marker: &str) -> Result<(usize, usize, Vec<&str>)> {
    let lines: Vec<&str> = log.lines().collect();
    let start = lines
        .iter()
        .position(|line| line.starts_with(start_marker))
        .with_context(|| format!("missing run marker {:?} in log", start_marker))?;
    let end = lines[start + 1..]
        .iter()
        .position(|line| line.starts_with(RUN_END_PREFIX))
        .with_context(|| format!("missing exit marker after {:?}", start_marker))?
        + start
        + 1;
    Ok((start, end, lines))
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let log_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("notes/bf-6bvq1-benchmark.log"));
    let out_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("notes/bf-6bvq1-metrics.json"));

    let log = fs::read_to_string(&log_path)
        .with_context(|| format!("reading benchmark log {}", log_path.display()))?;

    // Header provenance.
    let captured_at = header_field(&log, "date (UTC)").unwrap_or_default().to_string();
    let commit = header_field(&log, "commit").unwrap_or_default().to_string();
    let command = run1_command(&log).unwrap_or_default();

    // Run 3 (--progress-json appendix): machine-readable events + match lines.
    let (ev_start, ev_end, lines) = run_bounds(&log, RUN3_EVENTS_MARKER)?;
    let out_start = lines[ev_start + 1..ev_end]
        .iter()
        .position(|line| line.starts_with(RUN3_STDOUT_MARKER))
        .with_context(|| format!("missing {:?} marker in run 3 section", RUN3_STDOUT_MARKER))
        + ev_start
        + 1;
    let stderr_section = lines[ev_start + 1..out_start].join("\n");
    let stdout_section = lines[out_start + 1..ev_end].join("\n");

    let events = parse_progress_events(&stderr_section);
    let (stdout_lines, stdout_files) = count_match_lines(&stdout_section);

    // Run 2 (plain pipe): independent corroboration of run 3's counts.
    let (r2_start, r2_end, lines) = run_bounds(&log, "--- run 2:")?;
    let (run2_lines, run2_files) = count_match_lines(&lines[r2_start + 1..r2_end].join("\n"));

    // Run 1 summary: the tool's own throughput measurement.
    let (summary_files, summary_mib, displayed_secs, mib_per_sec, summary_line) =
        parse_searched_summary(&log)?;

    // Cross-checks — a mismatch means the parse is wrong; refuse to emit an artifact.
    ensure!(
        events.files_done > 0 && events.files_done == events.files_started,
        "file_done count {} != file_start count {}",
        events.files_done,
        events.files_started
    );
    ensure!(
        events.total_matches == stdout_lines,
        "file_done match sum {} != stdout match lines {}",
        events.total_matches,
        stdout_lines
    );
    ensure!(
        events.matched_files == stdout_files,
        "file_done matched files {} != stdout distinct files {}",
        events.matched_files,
        stdout_files
    );
    ensure!(
        summary_files == events.files_started,
        "summary file count {} != file_start count {}",
        summary_files,
        events.files_started
    );
    let events_mib = events.size_hint_sum as f64 / (1024.0 * 1024.0);
    ensure!(
        (summary_mib - events_mib).abs() <= 0.051,
        "summary total {:.1} MB vs size_hint sum {:.3} MiB disagree",
        summary_mib,
        events_mib
    );

    // Wall-clock runtime: the tool computed its throughput from an internal
    // millisecond duration that the capture did not record ("in 0.0s"), so
    // recover it from the exact byte total and the printed MiB/s.
    ensure!(
        mib_per_sec > 0.0,
        "summary throughput is not positive: {}",
        mib_per_sec
    );
    let wall_time_ms = ((events_mib / mib_per_sec) * 1000.0).round() as u128;

    // Reuse the benchmark plumbing for the structured extraction.
    let metrics: RawTimingMetrics = extract_raw_timing_metrics(
        &stdout_section,
        &stderr_section,
        wall_time_ms,
        events.size_hint_sum,
    );
    if let Err(error) = metrics.validate() {
        bail!("extracted raw timing metrics failed validation: {}", error);
    }
    ensure!(
        metrics.files_processed == events.files_started,
        "plumbing counted {} file_done events, parser counted {} file_start events",
        metrics.files_processed,
        events.files_started
    );
    ensure!(
        metrics.total_matches == events.total_matches,
        "plumbing summed {} matches, parser summed {}",
        metrics.total_matches,
        events.total_matches
    );

    let bytes_per_second = events.size_hint_sum as f64 / (wall_time_ms as f64 / 1000.0);
    let artifact = json!({
        "schema_version": 1,
        "source": {
            "bead": "bf-50mfjl",
            "log_path": "notes/bf-6bvq1-benchmark.log",
            "captured_at": captured_at,
            "commit": commit,
            "command": command,
            "progress_json_run": 3,
        },
        // Verbatim output of extract_raw_timing_metrics / RawTimingMetrics
        // (bf-5b8mk / bf-4b7pm plumbing in benches/grep_1000.rs).
        "raw_timing_metrics": serde_json::to_value(&metrics)?,
        "observed": {
            "files_total": events.files_started,
            "files_matched": events.matched_files,
            "total_matches": events.total_matches,
            "total_bytes": events.size_hint_sum,
            "bytes_per_second": bytes_per_second,
            "runtime": {
                "wall_time_ms": wall_time_ms,
                "derivation": format!(
                    "(total_bytes / 1 MiB) / {:.1} MiB/s * 1000 = {} ms",
                    mib_per_sec, wall_time_ms
                ),
                "note": "derived from the run 1 'Searched:' summary, the tool's own \
                         measurement; the log records the displayed duration only as \
                         'in 0.0s' (no per-run millisecond wall time was captured)"
            },
            "run1_summary": {
                "line": summary_line,
                "files": summary_files,
                "total_mib": summary_mib,
                "displayed_seconds": displayed_secs,
                "mib_per_second": mib_per_sec,
            },
            "run2_corroboration": {
                "match_lines": run2_lines,
                "files_matched": run2_files,
            },
        },
        "cross_checks": {
            "file_done_count_eq_file_start_count": events.files_done == events.files_started,
            "file_done_matches_eq_stdout_match_lines": events.total_matches == stdout_lines,
            "matched_files_eq_stdout_distinct_files": events.matched_files == stdout_files,
            "run1_summary_files_eq_files_total": summary_files == events.files_started,
            "run1_summary_mib_matches_size_hint_sum": (summary_mib - events_mib).abs() <= 0.051,
            "run2_eq_run3_match_lines": run2_lines == stdout_lines,
            "run2_eq_run3_files_matched": run2_files == stdout_files,
            "raw_timing_metrics_validation": "ok",
        },
    });

    let json = serde_json::to_string_pretty(&artifact)?;
    if let Some(parent) = out_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).with_context(|| {
                format!("creating output directory {}", parent.display())
            })?;
        }
    }
    fs::write(&out_path, json + "\n")
        .with_context(|| format!("writing metrics artifact {}", out_path.display()))?;

    println!(
        "Parsed {} ({} lines) -> {}",
        log_path.display(),
        log.lines().count(),
        out_path.display()
    );
    println!("  runtime:          {} ms (wall, derived)", metrics.wall_time_ms);
    println!("  files total:      {}", metrics.files_processed);
    println!("  files matched:    {}", events.matched_files);
    println!("  matches:          {}", metrics.total_matches);
    println!("  bytes processed:  {} ({:.3} MiB)", metrics.total_bytes, events_mib);
    println!("  files/sec:        {:.1}", metrics.files_per_second);
    println!("  throughput:       {:.1} MiB/s ({:.0} bytes/s)", metrics.throughput_mb_s, bytes_per_second);
    Ok(())
}
