# bf-25hf0: grep-corpus baseline metrics

## Task Completed

Captured grep-corpus baseline metrics to `benches/baselines/grep-corpus.json`.

## What Was Done

1. **Validated corpus exists**: Confirmed 1,000 PDF files in `tests/fixtures/grep-corpus/corpus/` totaling ~6.6 MB
2. **Measured corpus metrics**: 
   - File count: 1,000
   - Total size: 6,870,672 bytes (~6.6 MB)
3. **Recorded git metadata**:
   - Commit SHA: 77eeaecc
   - Timestamp: 2026-07-06T16:06:55+00:00
4. **Created baseline JSON**: `benches/baselines/grep-corpus.json` with available metrics

## Acceptance Criteria Status

- ✅ **PASS**: `benches/baselines/grep-corpus.json` exists
- ✅ **PASS**: JSON contains required fields (commit_sha, timestamp, corpus_size, bytes_total, notes)
- ✅ **PASS**: JSON is valid and parseable (verified with `python3 -m json.tool`)
- ✅ **PASS**: Corpus validation passes (1,000 files confirmed)
- ✅ **PASS**: Timestamp is current (2026-07-06T16:06:55+00:00)
- ✅ **PASS**: Commit hash matches HEAD (77eeaecc)
- ⚠️ **WARN**: Grep subcommand not yet implemented (blocked on 7.8.x beads), timing/throughput metrics deferred

## Notes

The grep functionality is not yet implemented in pdftract CLI. The benchmark code at `crates/pdftract-cli/benches/grep_1000.rs` is a TODO placeholder waiting for grep subcommand implementation (beads 7.8.x). 

This baseline captures the corpus state before implementation. Once grep is implemented, this baseline should be updated with actual timing metrics (throughput_mb_per_sec, files_per_sec, total_runtime_sec).

## Artifacts

- `benches/baselines/grep-corpus.json` - baseline metrics file
- Commit: a9a7eb0d (on main branch)

## Verification

```bash
# Validate JSON
cat benches/baselines/grep-corpus.json | python3 -m json.tool

# Verify corpus
ls -1 tests/fixtures/grep-corpus/corpus/*.pdf | wc -l  # 1000 files
du -sb tests/fixtures/grep-corpus/corpus/ | awk '{print $1}'  # 6870672 bytes
```