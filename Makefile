# pdftract Makefile
# Top-level build automation for pdftract project

.PHONY: help validate-corpus download-grep-corpus agentation-smoke mcp-verify readme-audit diagnostics-validate test clean

# Default target
help:
	@echo "pdftract Makefile targets:"
	@echo ""
	@echo "  validate-corpus       - Verify grep-corpus integrity against manifest"
	@echo "  download-grep-corpus   - Download/generate PDFs for grep-corpus"
	@echo "  test                   - Run all tests"
	@echo "  agentation-smoke       - Verify Agentation mounts in a browser"
	@echo "  mcp-verify             - Check the MCP catalog and all tool call contracts"
	@echo "  readme-audit           - Check README capability claims against evidence"
	@echo "  diagnostics-validate   - Validate the documented diagnostic registry and wire contracts"
	@echo "  clean                  - Clean build artifacts"
	@echo ""
	@echo "Corpus management:"
	@echo "  make validate-corpus                    # Validate existing corpus"
	@echo "  make download-grep-corpus               # Generate corpus (default: 1000 PDFs)"
	@echo "  make download-grep-corpus COUNT=500     # Generate specific count"
	@echo ""

# Validate corpus integrity against manifest.csv
validate-corpus:
	@echo "Validating grep-corpus..."
	@bash scripts/validate-corpus.sh tests/fixtures/grep-corpus

# Download/generate PDFs for grep-corpus
download-grep-corpus:
	@echo "Generating grep-corpus ($(COUNT) PDFs)..."
	@bash scripts/download-grep-corpus.sh $(COUNT)

# Build the real MCP server before checking its wire contract. This keeps the
# checker's per-frame timeout focused on protocol startup instead of a cold
# Cargo compilation. PDFTRACT_MCP_BIN may point at a prebuilt binary.
mcp-verify:
	cargo build --locked --package pdftract-cli --bin pdftract --features mcp
	python3 scripts/check-mcp-tool-catalog.py

# Verify every README capability/release marker against tracked evidence and
# the checkpoint state of any bead that keeps a claim in progress.
readme-audit:
	python3 scripts/audit_readme_capabilities.py

# Keep the published diagnostic-code catalog, Rust registry, and serialized
# diagnostic contracts synchronized in both local development and CI.
diagnostics-validate:
	@bash scripts/validate-diagnostic-registry.sh

# Run tests, including the real-binary MCP contract check.
test: mcp-verify
	cargo test --all-targets

# Run the browser smoke test for every HTML entry point
agentation-smoke:
	python3 scripts/agentation-smoke.py

# Clean build artifacts
clean:
	cargo clean
