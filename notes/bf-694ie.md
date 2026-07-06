# bf-694ie: Capture stdout and stderr from pdftract to a log file

## Summary
Verified that output redirection works properly with pdftract by capturing both stdout and stderr to log files.

## Tests Performed

### Test 1: Error output capture (stderr only)
```bash
pdftract extract tests/fixtures/sample.pdf > notes/bf-694ie-output.log 2>&1
```

**Result:**
- File created: `/home/coding/pdftract/notes/bf-694ie-output.log`
- File size: 29 bytes
- File permissions: `-rw-r--r--` (644 - readable by owner and group)
- Content: `Error: Failed to extract PDF`
- Exit code: 1

### Test 2: Help output capture (stdout only)
```bash
pdftract --help > notes/bf-694ie-help.log 2>&1
```

**Result:**
- File created: `/home/coding/pdftract/notes/bf-694ie-help.log`
- File size: 1,373 bytes
- File permissions: `-rw-r--r--` (644)
- Content: 26 lines of help text
- Exit code: 0

### Test 3: Hash command output capture
```bash
(pdftract hash tests/fixtures/test-minimal.pdf 2>&1) > notes/bf-694ie-hash.log
```

**Result:**
- File created: `/home/coding/pdftract/notes/bf-694ie-hash.log`
- File size: 47 bytes
- File permissions: `-rw-r--r--` (644)
- Content: `Error: Failed to compute fingerprint from file`
- Exit code: 2

## Verification

### File Creation
✅ All log files were created successfully in the `notes/` directory

### File Readability
✅ Files are readable with standard commands:
- `cat notes/bf-694ie-output.log` - SUCCESS
- `cat notes/bf-694ie-help.log` - SUCCESS
- `cat notes/bf-694ie-hash.log` - SUCCESS

### File Permissions
✅ All files have `-rw-r--r--` (644) permissions, allowing read access

### Content Capture
✅ Both stdout and stderr are captured:
- `--help` command (stdout) captured successfully
- Error messages (stderr) captured successfully

## Documented Redirection Syntax

The standard shell redirection syntax for capturing both stdout and stderr:

```bash
# Method 1: Redirect stderr to stdout, then to file (recommended)
pdftract <fixture-path> > output.log 2>&1

# Method 2: Alternate syntax
pdftract <fixture-path> &> output.log

# Method 3: Explicit redirection
pdftract <fixture-path> 1> output.log 2>&1
```

## Log File Locations
- `/home/coding/pdftract/notes/bf-694ie-output.log`
- `/home/coding/pdftract/notes/bf-694ie-help.log`
- `/home/coding/pdftract/notes/bf-694ie-hash.log`

## Acceptance Criteria Status

| Criterion | Status | Notes |
|-----------|--------|-------|
| stdout and stderr are both captured to a file | ✅ PASS | Verified with both `--help` (stdout) and error commands (stderr) |
| The log file is created in a known location | ✅ PASS | Files created in `/home/coding/pdftract/notes/` directory |
| The log file contains output from pdftract | ✅ PASS | All log files contain pdftract output (help text or error messages) |
| File permissions allow reading the log | ✅ PASS | All files have 644 permissions (-rw-r--r--) |
| The redirection method is documented | ✅ PASS | Documented three redirection methods above |

## Commits
- Created verification note: `notes/bf-694ie.md`

## Conclusion
The output redirection functionality works correctly with pdftract. Both stdout and stderr can be captured to log files using standard shell redirection (`> output.log 2>&1`). The log files are created with appropriate permissions and are readable with standard tools.
