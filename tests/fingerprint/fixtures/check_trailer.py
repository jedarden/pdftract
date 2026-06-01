#!/usr/bin/env python3
import pikepdf

# Dump the trailer for both files
print("=== v1 trailer ===")
with pikepdf.open("tests/fingerprint/fixtures/linearization_toggle/v1.pdf") as pdf:
    print(f"Trailer: {dict(pdf.trailer)}")
    print(f"/Root: {pdf.trailer.get('/Root')}")

print("\n=== v2 trailer ===")
with pikepdf.open("tests/fingerprint/fixtures/linearization_toggle/v2.pdf") as pdf:
    print(f"Trailer: {dict(pdf.trailer)}")
    print(f"/Root: {pdf.trailer.get('/Root')}")

# Read raw bytes to find the trailer
print("\n=== Raw v2 trailer (last 200 bytes) ===")
with open("tests/fingerprint/fixtures/linearization_toggle/v2.pdf", "rb") as f:
    f.seek(-200, 2)
    print(f.read())
