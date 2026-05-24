---
name: Performance regression
about: Report a slowdown or performance issue
title: '[PERF] '
labels: performance
assignees: ''
---

## Performance Issue Description

A clear and concise description of the performance problem.

## Baseline vs Current Performance

**BEFORE (working well):**
- Version: (e.g., 0.5.0)
- Processing time: (e.g., 2.5 seconds for a 100-page PDF)
- Memory usage: (e.g., 150 MB peak)

**AFTER (regression):**
- Version: (e.g., 0.6.0)
- Processing time: (e.g., 8 seconds for the same PDF)
- Memory usage: (e.g., 600 MB peak)

## Test Case

Please provide:
1. **PDF file** (attach or link to a representative file)
2. **Command used:**
   ```bash
   pdftract <command> <file>
   ```
3. **Benchmark results** (before and after):
   ```bash
   # Use `hyperfine` or similar for accurate measurements
   hyperfine 'pdftract old_version' 'pdftract new_version'
   ```

## Profiling Data (Optional but Helpful)

If available, attach profiling output:
```bash
# Flamegraph (Linux)
cargo install flamegraph
cargo flamegraph --bin pdftract -- <args>

# Instruments (macOS)
instruments -t "Time Profiler" cargo run --release -- <args>

# perf (Linux)
perf record -g cargo run --release -- <args>
perf report
```

## Environment

- **OS:** (e.g., Ubuntu 22.04, macOS 14, Windows 11)
- **Hardware:** (CPU, RAM - relevant for performance issues)
- **pdftract version:** (run `pdftract --version`)
- **Rust version:** (run `rustc --version`)

## Suspected Cause

If you have a hypothesis about what's causing the regression (e.g., a specific commit, a new dependency), please describe it here.

## Additional Context

Add any other context about the performance issue:

- Logs or traces
- Related issues or PRs
- Workarounds (e.g., using an older version)

---

**Note:** For help with development or contributing to pdftract, see [`CONTRIBUTING.md`](../../CONTRIBUTING.md).

- Logs or traces
- Related issues or PRs
- Workarounds (e.g., using an older version)
