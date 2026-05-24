# Pull Request

## Linked Issue or RFC

Closes #(issue number)

## Scope Statement

Which Phase / which Acceptance Scenario does this PR address?

- **Phase:** (e.g., Phase 2 - Font Encoding)
- **Acceptance Scenario:** (e.g., AS-2.3 - Embedded CMap with predefined CID->Unicode mapping)

## Summary

Brief description of what this PR does and why it's necessary.

## Changes Made

- List the main changes here
- Include file paths and key functions modified
- Note any breaking changes

## Test Plan

How did you verify this works?

- [ ] Unit tests pass (`cargo test --workspace --features default`)
- [ ] Integration tests pass
- [ ] Manual testing completed

### Test Evidence

Attach or paste:
- Terminal output from test runs
- Screenshots (for UI changes)
- Example PDF files processed (before/after)

## Performance Impact

If this PR touches hot-path code (parsing, text extraction, encoding resolution):

- [ ] No performance impact (CI changes, documentation, etc.)
- [ ] Performance improvement (include benchmarks)
- [ ] Performance regression (include justification)

### Benchmark Results (if applicable)

```text
(paste `cargo bench` output here)
```

## Checklist

- [ ] My code follows the style guidelines of this project (`cargo fmt`)
- [ ] I have performed a self-review of my code
- [ ] I have commented my code where necessary, particularly in hard-to-understand areas
- [ ] I have made corresponding changes to the documentation
- [ ] My changes generate no new warnings (`cargo clippy`)
- [ ] I have added tests that prove my fix is effective or that my feature works
- [ ] New and existing tests pass locally with `cargo test --workspace --features default`
- [ ] I have signed off my commits (`git commit -s`) per the DCO
- [ ] I have verified local validation per [`CONTRIBUTING.md`](../../CONTRIBUTING.md)

## Additional Notes

Any additional context, screenshots, or considerations for the reviewer.

---

**Note:** See [`CONTRIBUTING.md`](../../CONTRIBUTING.md) for development setup, local validation checklist, and commit message guidelines.
