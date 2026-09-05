//! Test error type imports from pdftract-py crate
//!
//! This test verifies that all PyO3 exception types can be imported
//! from the pdftract-py crate (library name "pdftract_py").

use std::any::TypeId;

use pdftract_py::{
    CorruptPdfError, EncryptionError, PdftractError, ReceiptVerifyError,
    RemoteFetchInterruptedError, SourceUnreachableError, TlsError, UnsupportedOperationError,
};

#[test]
fn test_error_types_import() {
    // Instantiating these PyO3 exceptions requires the Python GIL, so we
    // verify the imports by resolving each name to its `TypeId` instead.
    // Distinctness is what actually proves every name bound here refers to a
    // different concrete type — a mis-targeted `use` would collide here.
    let ids = [
        TypeId::of::<PdftractError>(),
        TypeId::of::<EncryptionError>(),
        TypeId::of::<CorruptPdfError>(),
        TypeId::of::<SourceUnreachableError>(),
        TypeId::of::<RemoteFetchInterruptedError>(),
        TypeId::of::<TlsError>(),
        TypeId::of::<ReceiptVerifyError>(),
        TypeId::of::<UnsupportedOperationError>(),
    ];

    for (i, a) in ids.iter().enumerate() {
        for b in ids.iter().skip(i + 1) {
            assert_ne!(a, b, "two error imports resolved to the same type");
        }
    }
}
