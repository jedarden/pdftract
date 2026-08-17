//! Test error type imports from pdftract-py crate
//!
//! This test verifies that all PyO3 exception types can be imported
//! from the pdftract-py crate (library name "pdftract_py").

use pdftract_py::{
    CorruptPdfError, EncryptionError, PdftractError, ReceiptVerifyError,
    RemoteFetchInterruptedError, SourceUnreachableError, TlsError, UnsupportedOperationError,
};

#[test]
fn test_error_types_import() {
    // This test just verifies that the imports compile
    // The actual error types are PyO3 exceptions and require
    // Python GIL to instantiate, so we just check compilation here
    assert!(true);
}
