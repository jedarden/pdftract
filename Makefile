# pdftract Makefile
# Top-level build automation for pdftract project

.PHONY: help validate-corpus download-grep-corpus agentation-smoke test clean

# Default target
help:
	@echo "pdftract Makefile targets:"
	@echo ""
	@echo "  validate-corpus       - Verify grep-corpus integrity against manifest"
	@echo "  download-grep-corpus   - Download/generate PDFs for grep-corpus"
	@echo "  test                   - Run all tests"
	@echo "  agentation-smoke       - Verify Agentation mounts in a browser"
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

# Run tests
test:
	cargo test --all-targets

# Run the browser smoke test for every HTML entry point
agentation-smoke:
	python3 scripts/agentation-smoke.py

# Clean build artifacts
clean:
	cargo clean
