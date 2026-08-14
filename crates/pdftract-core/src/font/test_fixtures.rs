//! Test fixtures module for font testing utilities.
//!
//! This module provides helper functions and fixtures for testing font-related
//! functionality across the pdftract crate. It aims to keep test helper code
//! organized and separate from production code.

use crate::font::encoding::FontEncoding;
use crate::graphics_state::Matrix3x3;

/// Placeholder function to verify compilation.
///
/// This function serves as a minimal verification that the test_fixtures module
/// is properly included and compiles successfully. It can be removed once actual
/// test fixtures are implemented.
pub fn _test_fixtures_loaded() -> bool {
    true
}

/// Creates a minimal valid FontEncoding for testing.
///
/// This function returns a FontEncoding with no base encoding set (None),
/// which is suitable for basic Type3Font construction in tests.
///
/// # Returns
///
/// A `FontEncoding` instance created with `FontEncoding::new(None)`.
///
/// # Use in Tests
///
/// ```rust
/// use pdftract_core::font::test_fixtures::font_encoding_default;
/// let encoding = font_encoding_default();
/// ```
pub fn font_encoding_default() -> FontEncoding {
    FontEncoding::new(None)
}

/// Creates an identity 3x3 transformation matrix for testing.
///
/// This function returns an identity matrix, which is the default
/// transformation matrix used in Type3Font construction when no specific
/// transformation is required.
///
/// # Returns
///
/// A `Matrix3x3` instance representing the identity transformation,
/// created with `Matrix3x3::identity()`.
///
/// # Use in Tests
///
/// ```rust
/// use pdftract_core::font::test_fixtures::matrix_identity;
/// let matrix = matrix_identity();
/// ```
pub fn matrix_identity() -> Matrix3x3 {
    Matrix3x3::identity()
}
