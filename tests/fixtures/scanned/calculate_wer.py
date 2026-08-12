#!/usr/bin/env python3
"""
Calculate Word Error Rate (WER) between ground truth and OCR output.

WER = (substitutions + insertions + deletions) / total_words_in_reference

Exit codes:
    0: WER ≤ 3% (passes quality gate)
    1: WER > 3% (fails quality gate)
    2: Error in calculation

Usage:
    python3 calculate_wer.py <ground_truth.txt> <ocr_output.txt> [--verbose]
"""

import sys
import argparse
from typing import List, Tuple


def normalize_text(text: str) -> str:
    """
    Normalize text for WER calculation.
    - Convert to lowercase
    - Remove extra whitespace
    - Remove common punctuation
    """
    # Convert to lowercase
    text = text.lower()

    # Remove common punctuation (but keep word separators)
    for punct in ['.', ',', '!', '?', ';', ':', '"', "'", '()', '[]', '{}']:
        text = text.replace(punct, ' ')

    # Normalize whitespace
    text = ' '.join(text.split())

    return text.strip()


def calculate_wer(reference: str, hypothesis: str) -> float:
    """
    Calculate Word Error Rate using Levenshtein distance.

    Args:
        reference: Ground truth text
        hypothesis: OCR output text

    Returns:
        WER as a float between 0.0 and 1.0
    """
    # Tokenize
    ref_words = reference.split()
    hyp_words = hypothesis.split()

    if len(ref_words) == 0:
        return 1.0 if len(hyp_words) > 0 else 0.0

    # Levenshtein distance for word sequences
    m, n = len(ref_words), len(hyp_words)

    # Initialize distance matrix
    dp = [[0] * (n + 1) for _ in range(m + 1)]

    # Initialize first row and column
    for i in range(m + 1):
        dp[i][0] = i
    for j in range(n + 1):
        dp[0][j] = j

    # Fill the matrix
    for i in range(1, m + 1):
        for j in range(1, n + 1):
            if ref_words[i - 1] == hyp_words[j - 1]:
                dp[i][j] = dp[i - 1][j - 1]
            else:
                dp[i][j] = 1 + min(
                    dp[i - 1][j],      # deletion
                    dp[i][j - 1],      # insertion
                    dp[i - 1][j - 1]   # substitution
                )

    # WER = edit_distance / reference_length
    wer = dp[m][n] / len(ref_words)
    return wer


def parse_args() -> argparse.Namespace:
    """Parse command line arguments."""
    parser = argparse.ArgumentParser(
        description='Calculate Word Error Rate (WER) between ground truth and OCR output',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Exit codes:
    0  WER ≤ 3%% (passes quality gate)
    1  WER > 3%% (fails quality gate)
    2  Error (missing files, etc.)

Example:
    python3 calculate_wer.py ground_truth.txt ocr_output.txt
        """
    )

    parser.add_argument('ground_truth', help='Path to ground truth text file')
    parser.add_argument('ocr_output', help='Path to OCR output text file')
    parser.add_argument('--verbose', '-v', action='store_true',
                       help='Enable verbose output with detailed statistics')

    return parser.parse_args()


def main() -> int:
    """Main entry point."""
    args = parse_args()

    try:
        # Read input files
        with open(args.ground_truth, 'r', encoding='utf-8') as f:
            ground_truth = f.read()

        with open(args.ocr_output, 'r', encoding='utf-8') as f:
            ocr_output = f.read()

    except FileNotFoundError as e:
        print(f"Error: File not found: {e}", file=sys.stderr)
        return 2
    except Exception as e:
        print(f"Error reading files: {e}", file=sys.stderr)
        return 2

    # Normalize texts
    ref_normalized = normalize_text(ground_truth)
    hyp_normalized = normalize_text(ocr_output)

    # Calculate WER
    wer = calculate_wer(ref_normalized, hyp_normalized)
    wer_percentage = wer * 100

    # Output results
    if args.verbose:
        ref_words = ref_normalized.split()
        hyp_words = hyp_normalized.split()

        print(f"Ground truth words: {len(ref_words)}")
        print(f"OCR output words: {len(hyp_words)}")
        print(f"Word Error Rate: {wer_percentage:.2f}%")
        print(f"Threshold: 3.00%")

        if wer <= 0.03:
            print("✓ PASS: WER ≤ 3%")
        else:
            print("✗ FAIL: WER > 3%")
    else:
        # Compact output for non-verbose mode
        print(f"WER: {wer_percentage:.2f}%")

    # Exit code based on WER threshold
    if wer <= 0.03:
        return 0
    else:
        return 1


if __name__ == '__main__':
    sys.exit(main())
