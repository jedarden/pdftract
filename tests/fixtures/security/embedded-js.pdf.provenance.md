# embedded-js.pdf fixture for TH-04 JavaScript presence testing
#
# PROVENANCE: synthetic, public-domain
#
# This PDF contains three embedded JavaScript actions designed to test that:
# 1. pdftract detects JavaScript presence (JAVASCRIPT_PRESENT diagnostic)
# 2. JavaScript is surfaced in metadata.javascript_actions for review
# 3. pdftract NEVER executes the JavaScript (security boundary)
#
# PDF Structure:
# - PDF Version: 1.4
# - Pages: 2
#
# JavaScript Actions (3 total):
# 1. Catalog /OpenAction → /JS containing app.alert("pwn")
#    Location: catalog.openaction
#    Purpose: Tests document-level JavaScript execution
#    Code: app.alert("pwn")
#
# 2. Page 0 /AA → /O (OnOpen action) → /JS containing app.alert('page_open')
#    Location: page.0.aa.o
#    Purpose: Tests page-level JavaScript execution on open
#    Code: app.alert('page_open')
#
# 3. Page 1 annotation /A → /JS containing app.alert('annot_action')
#    Location: page.1.annot.0.a
#    Purpose: Tests annotation-level JavaScript execution
#    Code: app.alert('annot_action')
#
# Test Verification:
# - Run extraction: pdftract extract embedded-js.pdf --json
# - Verify exactly 3 javascript_actions are present in output
# - Verify each action has correct location and code excerpt
# - Verify JAVASCRIPT_PRESENT diagnostic is emitted
# - Verify NO app.alert dialogs appear (no execution)
# - Verify process exit code is 0 (clean extraction)
#
# Security Properties:
# - All JavaScript is benign (only app.alert calls)
# - No network access, file I/O, or system commands
# - No exploit code or malicious payloads
# - Designed specifically to test DETECTION, not exploit behavior
#
# The fixture is safe to use in test environments because:
# - JavaScript content is synthetic and non-malicious
# - app.alert() only shows a dialog (harmless in non-interactive environments)
# - The test verifies NON-execution (no dialogs should appear)
# - pdftract does not include any JavaScript execution engine
# - Code review confirms no JS engine dependencies (boa, deno_core, v8, quickjs)
