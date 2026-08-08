#!/bin/bash
# Find all test function signatures and check for malformed ones

echo "=== Checking test function signatures ==="
echo ""

# Find test functions with parameters (anything other than empty parens)
echo "1. Test functions with NON-EMPTY parameters:"
grep -rn "^fn test_\|^    fn test_\|^        fn test_\|^            fn test_" /home/coding/pdftract --include="*.rs" | grep -v "fn test_[a-zA-Z_]*() " | grep -v "fn test_[a-zA-Z_]*()$" | grep -v "fn test_[a-zA-Z_]*(){" | grep -v "fn test_[a-zA-Z_]*() ->" | head -20

echo ""
echo "2. Test functions with explicit return types (should be rare):"
grep -rn "^fn test_\|^    fn test_\|^        fn test_\|^            fn test_" /home/coding/pdftract --include="*.rs" | grep " -> " | head -20

echo ""
echo "3. Checking for specific malformed patterns..."
echo ""
echo "   a. Functions with single parameter (e.g., fn test_foo(x: i32)):"
grep -rn "^fn test_\|^    fn test_\|^        fn test_\|^            fn test_" /home/coding/pdftract --include="*.rs" | grep -E "fn test_[a-zA-Z_]*\([^)]+:[^)]*\)" | head -10

echo ""
echo "   b. Functions with multiple parameters:"
grep -rn "^fn test_\|^    fn test_\|^        fn test_\|^            fn test_" /home/coding/pdftract --include="*.rs" | grep -E "fn test_[a-zA-Z_]*\([^,]+," | head -10
