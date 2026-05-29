# Sensitive fixture for TH-08 log audit testing
#
# PROVENANCE: synthetic, public-domain
#
# This PDF is password-protected with unique, distinctive markers designed
# to be unlikely to appear in normal log output. The test runs pdftract
# with RUST_LOG=trace and verifies that no sensitive content leaks into logs.
#
# PDF Contents:
# - Page 1 contains text: "UNIQUE-MARKER-IN-BODY-TEXT-7f9a"
# - Password: "UNIQUE-PASSWORD-FOR-TH08-7f9a"
# - Encryption: RC4-40 (V=1, R=2) for wide compatibility
#
# Test Verification:
# - Run pdftract extract with RUST_LOG=pdftract=trace
# - Capture stdout + stderr
# - Verify password value "UNIQUE-PASSWORD-FOR-TH08-7f9a" does NOT appear in logs
# - Verify body text "UNIQUE-MARKER-IN-BODY-TEXT-7f9a" does NOT appear in logs
# - Verify trace logging IS active (check for expected log patterns)
#
# The fixture is safe to use in test environments because:
# - The markers are synthetic and not real credentials
# - The password is only used for testing log leakage
# - The content is designed for substring-based leak detection
