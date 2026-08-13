#!/usr/bin/env python3
"""
Calculate Word Error Rate (WER) between ground truth and OCR output.

Usage:
    python3 calculate_wer.py <ground_truth.txt> <ocr_output.txt>

Requirements:
    pip3 install jiwer

For the degraded 200 DPI fixture, OCR output generation is documented at:
    notes/bf-3tedi.md

Example usage with degraded fixture:
    python3 tools/calculate_wer.py tests/fixtures/scanned/low-quality/degraded-200dpi-ground-truth.txt tests/fixtures/scanned/low-quality/degraded-200dpi-ocr.txt
"""

import sys
import argparse
from pathlib import Path

def calculate_wer_basic(ground_truth, hypothesis):
    """
    Calculate WER using basic Levenshtein distance.
    WER = (S + D + I) / N
    where S = substitutions, D = deletions, I = insertions, N = total words in reference
    """
    gt_words = ground_truth.strip().split()
    hyp_words = hypothesis.strip().split()

    if len(gt_words) == 0:
        return 1.0 if len(hyp_words) > 0 else 0.0

    # Dynamic programming for edit distance
    m, n = len(gt_words), len(hyp_words)
    dp = [[0] * (n + 1) for _ in range(m + 1)]

    for i in range(m + 1):
        dp[i][0] = i
    for j in range(n + 1):
        dp[0][j] = j

    for i in range(1, m + 1):
        for j in range(1, n + 1):
            if gt_words[i - 1] == hyp_words[j - 1]:
                dp[i][j] = dp[i - 1][j - 1]
            else:
                dp[i][j] = min(
                    dp[i - 1][j] + 1,      # deletion
                    dp[i][j - 1] + 1,      # insertion
                    dp[i - 1][j - 1] + 1   # substitution
                )

    return dp[m][n] / len(gt_words)


def calculate_cer_basic(ground_truth, hypothesis):
    """
    Calculate Character Error Rate (CER) using basic Levenshtein distance.
    CER = (S + D + I) / N
    where N = total characters in reference
    """
    gt_chars = list(ground_truth.strip())
    hyp_chars = list(hypothesis.strip())

    if len(gt_chars) == 0:
        return 1.0 if len(hyp_chars) > 0 else 0.0

    m, n = len(gt_chars), len(hyp_chars)
    dp = [[0] * (n + 1) for _ in range(m + 1)]

    for i in range(m + 1):
        dp[i][0] = i
    for j in range(n + 1):
        dp[0][j] = j

    for i in range(1, m + 1):
        for j in range(1, n + 1):
            if gt_chars[i - 1] == hyp_chars[j - 1]:
                dp[i][j] = dp[i - 1][j - 1]
            else:
                dp[i][j] = min(
                    dp[i - 1][j] + 1,      # deletion
                    dp[i][j - 1] + 1,      # insertion
                    dp[i - 1][j - 1] + 1   # substitution
                )

    return dp[m][n] / len(gt_chars)


def main():
    parser = argparse.ArgumentParser(description='Calculate WER/CER for OCR evaluation')
    parser.add_argument('ground_truth', help='Path to ground truth text file')
    parser.add_argument('hypothesis', help='Path to OCR output text file')
    parser.add_argument('--cer', action='store_true', help='Also calculate CER')
    parser.add_argument('--verbose', '-v', action='store_true', help='Verbose output')
    args = parser.parse_args()

    gt_path = Path(args.ground_truth)
    hyp_path = Path(args.hypothesis)

    if not gt_path.exists():
        print(f"Error: Ground truth file not found: {gt_path}", file=sys.stderr)
        sys.exit(1)

    if not hyp_path.exists():
        print(f"Error: Hypothesis file not found: {hyp_path}", file=sys.stderr)
        sys.exit(1)

    # Read files with error handling for encoding issues
    try:
        with open(gt_path, 'r', encoding='utf-8') as f:
            ground_truth = f.read()
    except UnicodeDecodeError:
        # Fallback to latin-1 for ground truth if UTF-8 fails
        with open(gt_path, 'r', encoding='latin-1') as f:
            ground_truth = f.read()

    try:
        with open(hyp_path, 'r', encoding='utf-8') as f:
            hypothesis = f.read()
    except UnicodeDecodeError:
        # Fallback to latin-1 for hypothesis (OCR output) if UTF-8 fails
        with open(hyp_path, 'r', encoding='latin-1') as f:
            hypothesis = f.read()

    wer = calculate_wer_basic(ground_truth, hypothesis)
    print(f"WER: {wer:.4f} ({wer * 100:.2f}%)")

    if args.cer:
        cer = calculate_cer_basic(ground_truth, hypothesis)
        print(f"CER: {cer:.4f} ({cer * 100:.2f}%)")

    if args.verbose:
        gt_words = ground_truth.strip().split()
        hyp_words = hypothesis.strip().split()
        print(f"\nReference words: {len(gt_words)}")
        print(f"Hypothesis words: {len(hyp_words)}")
        print(f"Reference chars: {len(ground_truth.strip())}")
        print(f"Hypothesis chars: {len(hypothesis.strip())}")

    # Return exit code based on WER threshold (3%)
    sys.exit(0 if wer < 0.03 else 1)


if __name__ == "__main__":
    main()
