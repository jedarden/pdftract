# Proptest Regressions

This directory contains minimal counterexamples discovered by proptest during CI runs.

Each file corresponds to a specific property test and contains the smallest input
that caused the test to fail. These files are committed to git so that:

1. Failures are reproducible across different machines
2. We can verify that fixes actually address the issue
3. We don't regress on previously-fixed bugs

## File Naming

Files are named `<test_name>.txt` where `<test_name>` is the full test path
with `/` replaced by `_`. For example:
- `proptest_lexer_prop_never_panics_on_random_bytes.txt`
- `proptest_object_parser_prop_parse_indirect_object_valid.txt`

## Usage

When proptest finds a failing case, it automatically writes the minimal
counterexample to this directory. On subsequent runs, proptest will first
test these known failures before generating new random inputs.

To reproduce a specific failure:
```bash
cargo test --features proptest -- proptest <test_name>
```

## Removing Files

Only remove a file from this directory if:
1. The underlying bug has been fixed AND
2. The test passes with the regression file present

Removing a regression file without fixing the bug will cause proptest to
re-discover the same failure on the next CI run.
