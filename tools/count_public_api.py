#!/usr/bin/env python3
"""Count public API coverage for pdftract-core - focus on re-exports in lib.rs."""

import re
from pathlib import Path

LIB_RS = Path("crates/pdftract-core/src/lib.rs")

# Parse lib.rs to find re-exports and public modules
with open(LIB_RS) as f:
    content = f.read()

# Public modules
pub_mods = re.findall(r'pub mod (\w+);', content)
print(f"Public modules ({len(pub_mods)}):")
for mod in pub_mods:
    print(f"  - {mod}")

# Re-exports
print("\nRe-exports:")
# pub use crate_name::item
re_exports = re.findall(r'pub use ([^:]+::(\w+(?:::\w+)*))', content)
for _, item in re_exports:
    print(f"  - {item}")

# Count unique public types from re-exports
print("\nKey public types to document:")
types = re.findall(r'pub use [^:]+::(\w+)', content)
unique_types = sorted(set(types))
for t in unique_types:
    print(f"  - {t}")
