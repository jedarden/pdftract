# XSS payload fixture for TH-09 testing
#
# PROVENANCE: synthetic, public-domain
#
# This PDF contains crafted text that resembles HTML/JavaScript XSS payloads:
#
# Page 1 contains four text spans:
# 1. <script>alert(1)</script>
# 2. <img src=x onerror="alert(2)">
# 3. javascript:alert(3)
# 4. <iframe src="javascript:alert(4)">
#
# These payloads are designed to test that:
# 1. The inspector renders extracted text as SVG <text> nodes (not innerHTML)
# 2. CSP headers (default-src 'self'; script-src 'self') are set on all responses
# 3. No script execution occurs even if the payloads are rendered
#
# The fixture is safe to use in test environments because:
# - The payloads are static text in the PDF content stream
# - The inspector's CSP prevents execution
# - The test verifies non-execution (window.__XSS_TRIGGERED__ remains undefined)
