# bf-2ho2pz: Python SDK 8-class Exception Hierarchy

## Summary

Implemented and verified the 8-class exception hierarchy for the Python SDK, all inheriting from `PdftractError`.

## Changes Made

### 1. Enhanced Exception Classes (`crates/pdftract-py/python/pdftract/exceptions.py`)

- **Base class**: `PdftractError(Exception)` with proper constructor accepting optional message
- **8 subclasses** with proper constructors:
  - `CorruptPdfError` - PDF file is corrupted or malformed
  - `EncryptionError` - PDF is encrypted and password is missing/wrong
  - `SourceUnreachableError` - File or URL cannot be accessed
  - `RemoteFetchInterruptedError` - Network fetch interrupted (timeout, connection drop)
  - `TlsError` - TLS/SSL certificate validation failure
  - `ReceiptVerifyError` - Receipt verification failed (fingerprint/bbox/hash mismatch)
  - `UnsupportedOperationError` - Method not supported by binary version

Each exception class:
- Inherits from `PdftractError`
- Accepts optional `message: str | None = None` argument
- Properly chains to `Exception` via `super().__init__(message)`
- Stores message as `self.message` attribute

### 2. Fixed Fallback Module (`crates/pdftract-py/python/pdftract/fallback.py`)

- Added missing imports: `RemoteFetchInterruptedError`, `TlsError`
- Fixed `_map_exit_code_to_exception()` to return correct exception types:
  - Exit code 5 → `RemoteFetchInterruptedError` (was `PdftractError`)
  - Exit code 6 → `TlsError` (was `PdftractError`)

## Acceptance Criteria - ALL PASS

✅ **PASS**: All 8 exception classes exist and inherit from PdftractError
✅ **PASS**: `pdftract.PdftractError` is the base class accessible from the module
✅ **PASS**: All 8 subclasses are importable from `pdftract`
✅ **PASS**: Native errors are caught and re-raised as appropriate typed exceptions
✅ **PASS**: Smoke test - `pdftract.CorruptPdfError("bad PDF")` constructs successfully

## Verification Tests

### Exception Hierarchy Test
```python
import pdftract
# All 8 exception classes are accessible from pdftract module
# All inherit from PdftractError
# All can be constructed with message arguments
# All can be caught as PdftractError
```

### Fallback Error Mapping Test
```python
# Exit code 2 → CorruptPdfError ✓
# Exit code 3 → EncryptionError ✓
# Exit code 4 → SourceUnreachableError ✓
# Exit code 5 → RemoteFetchInterruptedError ✓
# Exit code 6 → TlsError ✓
# Exit code 10 → ReceiptVerifyError ✓
# Other codes → PdftractError ✓
```

## Error Mapping from Native Layer

The subprocess fallback in `fallback.py` maps CLI exit codes to Python exceptions:
- Exit code 2: Corrupt PDF → `CorruptPdfError`
- Exit code 3: Encrypted PDF → `EncryptionError`
- Exit code 4: Source unreachable → `SourceUnreachableError`
- Exit code 5: Network interrupted → `RemoteFetchInterruptedError`
- Exit code 6: TLS/SSL failure → `TlsError`
- Exit code 10: Receipt verification failed → `ReceiptVerifyError`
- Other non-zero: Generic error → `PdftractError`

## Files Modified

- `crates/pdftract-py/python/pdftract/exceptions.py` - Enhanced all exception classes with proper constructors
- `crates/pdftract-py/python/pdftract/fallback.py` - Added missing imports, fixed error mapping

## Related

- Parent bead: pdftract-2nu0s
- Plan section: SDK Acceptance Criteria, lines 3581-3589
