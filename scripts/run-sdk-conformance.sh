#!/usr/bin/env bash
# Run one SDK conformance runner while preserving its report and exit status.
#
# The Argo matrix deliberately continues after an individual runner fails so
# that every SDK produces a diagnostic artifact. The final validator consumes
# the exit-code marker and report emitted here and turns the matrix red.

set -u

if [ "$#" -lt 4 ]; then
    echo "usage: $0 <sdk> <report-path> <exit-code-path> <command> [args...]" >&2
    exit 2
fi

sdk=$1
report_path=$2
exit_code_path=$3
shift 3

mkdir -p "$(dirname "$report_path")" "$(dirname "$exit_code_path")"

runner_exit=0
"$@" || runner_exit=$?

# A crash before the runner can write its report must remain an inspectable,
# schema-shaped failure artifact rather than an absent artifact.
if [ ! -s "$report_path" ]; then
    timestamp=$(date -u '+%Y-%m-%dT%H:%M:%SZ')
    cat >"$report_path" <<EOF
{
  "sdk": "$sdk",
  "sdk_version": "0.0.0",
  "suite_version": "0.0.0",
  "schema_version": "0.0",
  "timestamp": "$timestamp",
  "results": [
    {
      "id": "runner-startup",
      "status": "error",
      "error": "runner exited before emitting conformance-report.json",
      "duration_ms": 0
    }
  ],
  "summary": {
    "total": 1,
    "passed": 0,
    "failed": 0,
    "skipped": 0,
    "errors": 1,
    "duration_ms": 0
  }
}
EOF
    [ "$runner_exit" -eq 0 ] && runner_exit=1
fi

printf '%s\n' "$runner_exit" >"$exit_code_path"
echo "SDK conformance runner '$sdk' exited with code $runner_exit"
echo "Report: $report_path"

exit "$runner_exit"
