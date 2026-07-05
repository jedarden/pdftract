#!/usr/bin/env python3
"""
Competitive benchmark runner for pdftract.

Drives all competitor benchmarks (pdftract, pdfminer.six, pypdf, pdfplumber)
against identical PDF fixtures and emits per-commit throughput results to
benches/results/<commit-sha>.json.

Usage:
    python3 benches/competitors/run_all.py

Requirements (from benches/competitors/requirements.txt):
    pdfminer.six==20231228
    pypdf==4.2.0
    pdfplumber==0.11.0

Output format:
    {
      "commit": "<sha>",
      "date": "<iso-timestamp>",
      "pdftract_mbs": <float>,
      "pdfminer_mbs": <float>,
      "pypdf_mbs": <float>,
      "pdfplumber_mbs": <float>,
      "ratio_pdfminer": <float>,
      "ratio_pypdf": <float>
    }
"""

import json
import os
import subprocess
import sys
import time
from pathlib import Path
from datetime import datetime
from typing import Dict, List, Optional

# Script directory
SCRIPT_DIR = Path(__file__).parent.resolve()
# Repository root
REPO_ROOT = SCRIPT_DIR.parent.parent
# Results directory
RESULTS_DIR = REPO_ROOT / "benches" / "results"
# Competitor corpus directory
CORPUS_DIR = SCRIPT_DIR / "corpus"
# Virtualenv marker for requirements.txt validation
VENV_MARKER = SCRIPT_DIR / ".venv_installed"

# Required tools mapping: name -> requirements.txt package name
TOOLS = {
    "pdftract": None,  # System binary
    "pdfminer": "pdfminer.six",
    "pypdf": "pypdf",
    "pdfplumber": "pdfplumber",
}

# Wrapper scripts for each tool
WRAPPERS = {
    "pdfminer": SCRIPT_DIR / "run-pdfminer.sh",
    "pypdf": SCRIPT_DIR / "run-pypdf.sh",
    "pdfplumber": SCRIPT_DIR / "run-pdfplumber.sh",
    "pdftract": SCRIPT_DIR / "run-pdftract.sh",
}

# Acceptance thresholds (from plan Tier 4 targets)
THRESHOLD_PDFMINER_RATIO = 10.0
THRESHOLD_PYPDF_RATIO = 5.0


def find_pip_command() -> Optional[str]:
    """Find an available pip command."""
    pip_commands = ["pip", "pip3", "python3 -m pip", "python -m pip"]

    for cmd in pip_commands:
        try:
            result = subprocess.run(
                cmd.split() if not cmd.startswith("python") else ["python3", "-m", "pip"],
                capture_output=True,
                text=True,
            )
            if "--version" in cmd or result.returncode == 0:
                # Verify it actually works
                result = subprocess.run(
                    (*cmd.split(), "--version") if " " in cmd else [cmd, "--version"],
                    capture_output=True,
                    text=True,
                )
                if "pip" in result.stdout.lower():
                    return cmd
        except (FileNotFoundError, subprocess.SubprocessError):
            continue

    return None


def setup_virtualenv() -> None:
    """Ensure virtualenv is set up with required packages."""
    requirements_file = SCRIPT_DIR / "requirements.txt"

    if not requirements_file.exists():
        print(f"ERROR: requirements.txt not found at {requirements_file}", file=sys.stderr)
        sys.exit(1)

    # Check if packages are already installed
    try:
        import pdfminer.high_level
        import pypdf
        import pdfplumber

        print("✓ Competitor tools already installed")
        return
    except ImportError:
        print("Competitor tools not installed, attempting installation...")

    # Find a working pip command
    pip_cmd = find_pip_command()
    if not pip_cmd:
        print(
            "WARNING: pip not found. Competitor tools must be installed manually.",
            file=sys.stderr,
        )
        print(f"Install with: pip install -r {requirements_file}", file=sys.stderr)
        print("If running in CI, packages will be pre-installed in the container.", file=sys.stderr)
        sys.exit(1)

    print(f"Installing competitor tools using: {pip_cmd}")
    cmd = (*pip_cmd.split(), "install", "--no-cache-dir", "-r", str(requirements_file))

    result = subprocess.run(
        cmd,
        capture_output=True,
        text=True,
    )

    if result.returncode != 0:
        print(f"ERROR: Failed to install requirements", file=sys.stderr)
        print(f"stdout: {result.stdout}", file=sys.stderr)
        print(f"stderr: {result.stderr}", file=sys.stderr)
        sys.exit(1)

    # Mark as installed
    VENV_MARKER.touch()
    print("✓ Competitor tools installed")


def get_corpus_pdfs() -> List[Path]:
    """Get all PDF files from the corpus directory."""
    if not CORPUS_DIR.exists():
        print(f"ERROR: Corpus directory not found: {CORPUS_DIR}", file=sys.stderr)
        sys.exit(1)

    pdfs = sorted(CORPUS_DIR.glob("*.pdf"))
    if not pdfs:
        print(f"ERROR: No PDF files found in {CORPUS_DIR}", file=sys.stderr)
        sys.exit(1)

    print(f"Found {len(pdfs)} PDF files in corpus")
    return pdfs


def get_file_size_mb(path: Path) -> float:
    """Get file size in megabytes."""
    return path.stat().st_size / (1024 * 1024)


def run_extraction_tool(tool: str, pdf_path: Path) -> float:
    """
    Run extraction tool on a PDF and measure throughput in MB/s.

    Args:
        tool: Tool name (pdftract, pdfminer, pypdf, pdfplumber)
        pdf_path: Path to PDF file

    Returns:
        Throughput in MB/s
    """
    wrapper = WRAPPERS.get(tool)
    if not wrapper or not wrapper.exists():
        print(f"WARNING: Wrapper not found for {tool}: {wrapper}", file=sys.stderr)
        return 0.0

    file_size_mb = get_file_size_mb(pdf_path)
    if file_size_mb == 0:
        return 0.0

    # Run the wrapper and time it
    start = time.monotonic()
    try:
        result = subprocess.run(
            [str(wrapper), str(pdf_path)],
            capture_output=True,
            text=True,
            timeout=60,  # 60 second timeout per extraction
        )
    except subprocess.TimeoutExpired:
        print(f"WARNING: {tool} timed out on {pdf_path.name}", file=sys.stderr)
        return 0.0

    elapsed = time.monotonic() - start

    if result.returncode != 0:
        print(f"WARNING: {tool} failed on {pdf_path.name} (exit code {result.returncode})", file=sys.stderr)
        if result.stderr:
            print(f"  stderr: {result.stderr[:200]}", file=sys.stderr)
        return 0.0

    # Calculate throughput: MB / seconds
    throughput = file_size_mb / elapsed if elapsed > 0 else 0.0
    return throughput


def benchmark_tool(tool: str, pdfs: List[Path]) -> List[float]:
    """
    Benchmark a tool across all PDFs.

    Args:
        tool: Tool name
        pdfs: List of PDF paths

    Returns:
        List of throughput measurements (MB/s) for each PDF
    """
    print(f"\n=== Benchmarking {tool} ===")
    throughputs = []

    for i, pdf_path in enumerate(pdfs, 1):
        print(f"[{i}/{len(pdfs)}] {pdf_path.name}...", end=" ")
        sys.stdout.flush()

        throughput = run_extraction_tool(tool, pdf_path)
        throughputs.append(throughput)

        if throughput > 0:
            print(f"{throughput:.2f} MB/s")
        else:
            print("FAILED")

    return throughputs


def calculate_median(values: List[float]) -> float:
    """Calculate median of a list of values, excluding zeros."""
    # Filter out zeros (failed runs)
    valid = [v for v in values if v > 0]
    if not valid:
        return 0.0

    sorted_values = sorted(valid)
    n = len(sorted_values)
    mid = n // 2

    if n % 2 == 0:
        return (sorted_values[mid - 1] + sorted_values[mid]) / 2.0
    else:
        return sorted_values[mid]


def get_git_info() -> tuple[str, str]:
    """Get current git commit SHA and date."""
    try:
        commit = subprocess.check_output(
            ["git", "rev-parse", "HEAD"],
            cwd=REPO_ROOT,
            text=True,
        ).strip()
    except subprocess.CalledProcessError:
        commit = "unknown"

    # Get commit date
    try:
        date_str = subprocess.check_output(
            ["git", "show", "-s", "--format=%ci", "HEAD"],
            cwd=REPO_ROOT,
            text=True,
        ).strip()
        # Parse and convert to ISO format
        commit_date = datetime.strptime(date_str, "%Y-%m-%d %H:%M:%S %z")
        commit_date = commit_date.strftime("%Y-%m-%dT%H:%M:%SZ")
    except (subprocess.CalledProcessError, ValueError):
        commit_date = datetime.utcnow().strftime("%Y-%m-%dT%H:%M:%SZ")

    return commit, commit_date


def check_acceptance_criteria(results: Dict[str, float]) -> bool:
    """
    Check if results meet acceptance criteria.

    Args:
        results: Dict with median throughput per tool

    Returns:
        True if all criteria pass
    """
    pdftract_mbs = results.get("pdftract_mbs", 0.0)
    pdfminer_mbs = results.get("pdfminer_mbs", 0.0)
    pypdf_mbs = results.get("pypdf_mbs", 0.0)

    # Calculate ratios
    ratio_pdfminer = pdftract_mbs / pdfminer_mbs if pdfminer_mbs > 0 else 0.0
    ratio_pypdf = pdftract_mbs / pypdf_mbs if pypdf_mbs > 0 else 0.0

    print("\n=== Acceptance Criteria ===")
    passed = True

    # Check pdfminer ratio
    print(f"pdftract vs pdfminer.six: {ratio_pdfminer:.1f}x (target: ≥ {THRESHOLD_PDFMINER_RATIO}x)", end=" ")
    if ratio_pdfminer >= THRESHOLD_PDFMINER_RATIO:
        print("✓ PASS")
    else:
        print("✗ FAIL")
        passed = False

    # Check pypdf ratio
    print(f"pdftract vs pypdf: {ratio_pypdf:.1f}x (target: ≥ {THRESHOLD_PYPDF_RATIO}x)", end=" ")
    if ratio_pypdf >= THRESHOLD_PYPDF_RATIO:
        print("✓ PASS")
    else:
        print("✗ FAIL")
        passed = False

    return passed


def main() -> int:
    """Run all competitive benchmarks and emit results."""
    print("=" * 60)
    print("Competitive Benchmark Runner")
    print("=" * 60)

    # Setup virtualenv
    setup_virtualenv()

    # Get corpus PDFs
    pdfs = get_corpus_pdfs()

    # Benchmark each tool
    all_results = {}
    for tool in TOOLS.keys():
        throughputs = benchmark_tool(tool, pdfs)
        median_throughput = calculate_median(throughputs)
        key = f"{tool}_mbs"
        all_results[key] = median_throughput
        print(f"Median throughput: {median_throughput:.2f} MB/s")

    # Calculate ratios
    pdftract_mbs = all_results.get("pdftract_mbs", 0.0)
    pdfminer_mbs = all_results.get("pdfminer_mbs", 0.0)
    pypdf_mbs = all_results.get("pypdf_mbs", 0.0)

    all_results["ratio_pdfminer"] = (
        pdftract_mbs / pdfminer_mbs if pdfminer_mbs > 0 else 0.0
    )
    all_results["ratio_pypdf"] = pdftract_mbs / pypdf_mbs if pypdf_mbs > 0 else 0.0

    # Get git info
    commit, commit_date = get_git_info()
    all_results["commit"] = commit
    all_results["date"] = commit_date

    # Create results directory
    RESULTS_DIR.mkdir(parents=True, exist_ok=True)

    # Write results file
    results_file = RESULTS_DIR / f"{commit}.json"
    with open(results_file, "w") as f:
        json.dump(all_results, f, indent=2)

    print(f"\n✓ Results written to {results_file.relative_to(REPO_ROOT)}")

    # Check acceptance criteria
    passed = check_acceptance_criteria(all_results)

    if not passed:
        print("\n✗ Acceptance criteria FAILED")
        return 1

    print("\n✓ All acceptance criteria PASSED")
    return 0


if __name__ == "__main__":
    sys.exit(main())
