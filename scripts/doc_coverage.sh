#!/usr/bin/env bash
# Script to measure rustdoc coverage for pdftract-core

cd /home/coding/pdftract || exit 1

# Find all public items (pub fn, pub struct, pub enum, pub trait, pub mod, pub type, pub const)
# Count lines with pub declarations
TOTAL_ITEMS=$(grep -rn '^pub ' crates/pdftract-core/src --include='*.rs' 2>/dev/null | wc -l)

# Find doc comments (/// or //!)
DOC_COMMENTS=$(grep -rn '^////' crates/pdftract-core/src --include='*.rs' 2>/dev/null | wc -l)

# This is a rough estimate; we need a more sophisticated tool
echo "Public item declarations: $TOTAL_ITEMS"
echo "Doc comment lines: $DOC_COMMENTS"
echo "Note: This is a rough count. Real coverage needs rustdoc analysis."

# For better coverage, we'll use cargo-deadlinks or similar tools
# For now, let's just build the docs and see what happens
