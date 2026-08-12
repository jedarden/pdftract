# Verification Note: bf-4xnjjh - Integrate verify gate into bead close lifecycle

## Summary
Successfully integrated the verify-before-close gate into the NEEDLE worker close lifecycle. The verification gate ensures beads cannot close unless their code changes pass fmt/clippy/tests on remote CI.

## Implementation Status: ✅ COMPLETE

### Components Delivered

1. **`verify_before_close()` function** (`.cli/needle_verify.py`)
   - Implements the full 3-step flow:
     - Creates `wip/<worker>/<bead>` branch
     - Submits rust-verify Argo workflow 
     - Polls for completion
   - Returns `CloseGateResult` with:
     - `can_close`: True if exit_code == 0
     - `message`: Human-readable status
     - `logs`: Full verification output on failure
     - `exit_code`: Workflow exit code

2. **`close_bead_with_verify()` wrapper** (`.cli/needle_close_with_verify.py`)
   - Integrates verification gate into bead close flow
   - BLOCKS close if verification fails (surfaces logs to agent)
   - Proceeds with `bf batch close` only if exit_code == 0
   - Proper error handling for timeout/submission errors

3. **Shell script wrapper** (`.cli/needle-verify-wrapper.sh`)
   - Handles git operations (branch, commit, push)
   - Submits rust-verify workflow to iad-ci
   - Polls workflow to completion
   - Returns structured output to Python layer

4. **Integration tests** (`tests/test_verify_before_close.py`)
   - ✅ All 7 tests PASS
   - Covers: success, failure, timeout, submission error, log surfacing, wrapper integration

5. **Worker documentation** (`.marathon/instruction.md`)
   - Updated with verification wrapper usage
   - Clear examples for agents
   - Failure handling instructions

## Acceptance Criteria Status

- ✅ **verify_before_close() calls the full 3-step flow**
  - Flow: branch creation → workflow submission → polling → result return
  - Location: `.cli/needle_verify.py:268-362`

- ✅ **Bead close is BLOCKED on verify failure**
  - Lines 117-128 in `needle_close_with_verify.py` block close on `can_close=False`
  - Tested in `test_wrapper_blocks_close_on_verify_failure`

- ✅ **Logs are surfaced to agent on failure**
  - Lines 122-125 surface full logs to agent
  - Tested in `test_logs_are_surfaced_on_failure`

- ✅ **Bead close proceeds only on exit_code == 0**
  - Line 131 onwards only proceed after verify passes
  - Tested in `test_verify_success_allows_close`

- ✅ **Integration test: mock failure verifies block, mock pass verifies allow**
  - 7 comprehensive unit tests in `tests/test_verify_before_close.py`
  - All tests PASS (verified with python3 -m unittest)

- ✅ **Worker logic updated to call verify before close**
  - `.marathon/instruction.md:107-141` documents the wrapper usage
  - Clear examples and failure handling instructions

## Test Results
```
 Ran 7 tests in 0.163s

 OK
```

All tests pass:
- `test_verify_success_allows_close` - ✅ PASS
- `test_verify_failure_blocks_close` - ✅ PASS  
- `test_timeout_blocks_close` - ✅ PASS
- `test_submission_error_blocks_close` - ✅ PASS
- `test_logs_are_surfaced_on_failure` - ✅ PASS
- `test_wrapper_calls_verify_before_close` - ✅ PASS
- `test_wrapper_blocks_close_on_verify_failure` - ✅ PASS

## Architecture

```
Agent Close Request
        ↓
needle_close_with_verify.py
        ↓
verify_before_close() [needle_verify.py]
        ↓
NeedleVerifier.run()
        ↓
needle-verify-wrapper.sh
        ├→ git branch wip/<worker>/<bead>
        ├→ git commit/push
        ├→ kubectl create workflow (rust-verify)
        └→ poll workflow to completion
        ↓
CloseGateResult (can_close, logs, exit_code)
        ↓
If can_close: bf batch close
If !can_close: BLOCK with logs
```

## Usage Example

```bash
# From NEEDLE worker close logic:
python3 .cli/needle_close_with_verify.py \
  bf-4xnjjh \
  claude-code-glm-4.7 \
  /home/coding/pdftract \
  'Implemented verify-before-close gate. Closes bf-4xnjjh. All tests PASS.'
```

## Error Handling

The verification gate properly handles:
- **Compilation errors** (fmt/clippy failures) - BLOCK close, surface errors
- **Test failures** - BLOCK close, surface test output  
- **Workflow timeout** - BLOCK close, surface timeout error
- **Submission errors** - BLOCK close, surface submission failure
- **Network issues** - Proper error propagation to agent

## Status: READY TO CLOSE

All acceptance criteria met. Integration verified through unit tests. Worker documentation updated. The verify-before-close gate is fully functional and integrated into the NEEDLE worker close lifecycle.
