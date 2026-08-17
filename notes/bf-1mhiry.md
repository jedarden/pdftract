# Regex Catastrophic Backtracking Analysis (Bead bf-1mhiry)

## Executive Summary

This analysis examined **36 regex patterns** across the pdftract codebase for catastrophic backtracking vulnerabilities. **6 patterns were identified as HIGH or CRITICAL severity** due to accepting user input without proper validation or escaping.

## Vulnerability Categories

### CRITICAL Severity (3 patterns)

These patterns accept direct user input and are vulnerable to catastrophic backtracking DoS attacks:

#### 1. `crates/pdftract-core/examples/search.rs:32`
- **Pattern:** `format!("(?i){}", pattern)` where `pattern` is user input
- **Input:** Direct user input (command-line argument)
- **Vulnerability:** No validation or escaping; user can inject malicious regex patterns like `((a+)+)`
- **Complexity:** Exponential (O(2^n)) - can cause catastrophic backtracking on crafted input
- **Attack Vector:** User supplies pattern like `((a+)+)b` which becomes `(?i)((a+)+)b` and causes exponential backtracking on input like `"aaaaaaaaaaaaaaaaaaaaaaab"`
- **Mitigation:** 
  - Use `regex::escape()` on user input before interpolation
  - Implement pattern validation whitelist
  - Add timeout/size limits on pattern compilation

#### 2. `crates/pdftract-core/src/sdk.rs:248`
- **Pattern:** `format!("(?i){}", search_pattern)` 
- **Input:** User search pattern (can be escaped or unescaped depending on code path)
- **Vulnerability:** Inconsistent escaping - line 236 uses `regex::escape()` but line 248 does not
- **Complexity:** Exponential (O(2^n)) - varies based on user input
- **Attack Vector:** User supplies malicious pattern through case-insensitive search path
- **Mitigation:** Ensure all user input paths use `regex::escape()` before `(?i)` interpolation

#### 3. `crates/pdftract-cli/src/mcp/tools/registry.rs:724`
- **Pattern:** `Regex::new(&tool_args.pattern)`
- **Input:** Direct user input from MCP tool call
- **Vulnerability:** Only validates compilation, not pattern safety; allows malicious patterns
- **Complexity:** Exponential (O(2^n)) - depends on user-supplied pattern
- **Attack Vector:** MCP client supplies malicious pattern like `(a+)+b`
- **Mitigation:**
  - Implement pattern validation and sanitization
  - Use regex-safe escapes or pattern whitelist
  - Add compilation timeout and pattern complexity limits

### HIGH Severity (1 pattern)

#### 4. `crates/pdftract-core/src/sdk.rs:239`
- **Pattern:** `Regex::new(&search_pattern)` where `search_pattern` may use `regex::escape()`
- **Input:** User search pattern (partially escaped)
- **Vulnerability:** While line 236 uses `regex::escape()`, the case-insensitive path (line 248) bypasses this
- **Complexity:** Linear to exponential depending on execution path
- **Mitigation:** Ensure consistent escaping across all code paths

### MEDIUM Severity (3 patterns)

These patterns use configuration-controlled input, making them vulnerable to malicious configuration:

#### 5. `crates/pdftract-core/src/profiles/match_eval.rs:367`
- **Pattern:** `Regex::new(pattern)` with pattern from profile configuration
- **Input:** Profile configuration patterns (admin-controlled)
- **Vulnerability:** Malicious profile configuration can inject dangerous patterns
- **Complexity:** Depends on configured pattern
- **Mitigation:** Add pattern validation for profile configurations

#### 6. `crates/pdftract-core/src/profiles/engine.rs:486`
- **Pattern:** `Regex::new(pattern)` from profile configuration
- **Input:** Profile configuration patterns (admin-controlled)
- **Vulnerability:** Accepts any valid regex from configuration without validation
- **Complexity:** Depends on configured pattern
- **Mitigation:** Implement pattern validation and complexity limits

#### 7. `crates/pdftract-core/src/profiles/field_extractor.rs:131`
- **Pattern:** `Regex::new(pattern)` from profile configuration  
- **Input:** Profile configuration patterns (admin-controlled)
- **Vulnerability:** No validation of configuration patterns
- **Complexity:** Depends on configured pattern
- **Mitigation:** Add pattern validation to profile loading

### LOW Severity (30 patterns)

Fixed patterns with reasonable complexity and no user input:

1. **Fingerprint validation:** `r"^pdftract-v1:[0-9a-f]{64}$"` - O(n) linear
2. **List detection:** `r"^\s*[•‣◦⁃\-\*]\s"`, `r"^\s*\d+[.\)]\s"` - O(n) linear
3. **Log redaction patterns:** Various patterns for sensitive data redaction - O(n) to O(n*m) where m is pattern length
4. **Signal detection patterns:** Currency, date, invoice patterns - O(n) linear
5. **Other fixed patterns:** - O(n) linear complexity

These fixed patterns are safe because:
- No user input
- No nested quantifiers
- No overlapping alternation with shared prefixes
- Bounded repetition with fixed limits

## Risk Matrix

| Location | Input Type | Severity | Complexity | Vulnerability Type | Mitigation Priority |
|----------|------------|----------|------------|-------------------|---------------------|
| `examples/search.rs:32` | User Input | **CRITICAL** | O(2^n) Exponential | Direct injection, no escaping | **P0 - Immediate** |
| `src/sdk.rs:248` | User Input | **CRITICAL** | O(2^n) Exponential | Inconsistent escaping | **P0 - Immediate** |
| `mcp/tools/registry.rs:724` | User Input | **CRITICAL** | O(2^n) Exponential | No pattern validation | **P0 - Immediate** |
| `src/sdk.rs:239` | User Input | HIGH | O(n) to O(2^n) | Partial escaping | **P1 - High** |
| `src/profiles/match_eval.rs:367` | Config | MEDIUM | Pattern-dependent | No config validation | **P2 - Medium** |
| `src/profiles/engine.rs:486` | Config | MEDIUM | Pattern-dependent | No config validation | **P2 - Medium** |
| `src/profiles/field_extractor.rs:131` | Config | MEDIUM | Pattern-dependent | No config validation | **P2 - Medium** |
| Fixed patterns (30) | None | LOW | O(n) Linear | None | P3 - Low |

## Catastrophic Backtracking Attack Examples

### Attack Pattern 1: Nested Quantifiers
```regex
((a+)+)+b
```
**Input:** `aaaaaaaaaaaaaaaaaaaaaaac`  
**Effect:** Exponential backtracking trying all ways to group `a`'s before failing to match `b`

### Attack Pattern 2: Overlapping Alternation  
```regex
(a|a)+b
```
**Input:** `aaaaaaaaaaaaaaaaaaaaaaac`  
**Effect:** Multiple ways to match each `a` causes exponential behavior

### Attack Pattern 3: Unbounded Repetition
```regex
(.*)*\d+  
```
**Input:** Long string without digits  
**Effect:** Tries all possible ways to split the string

## Recommended Mitigations

### For CRITICAL Severity Patterns (User Input)

1. **Implement `regex::escape()` consistently**
   ```rust
   // Before (vulnerable):
   let regex = Regex::new(&format!("(?i){}", user_input))?;
   
   // After (safe):
   let escaped = regex::escape(&user_input);
   let regex = Regex::new(&format!("(?i){}", escaped))?;
   ```

2. **Add pattern validation whitelist**
   ```rust
   fn validate_pattern(pattern: &str) -> Result<()> {
       // Reject patterns with dangerous constructs
       if pattern.contains("(") && pattern.contains(")+") {
           return Err(Error::invalid_pattern("nested quantifiers"));
       }
       // More validation...
       Ok(())
   }
   ```

3. **Add timeout and size limits**
   ```rust
   // Limit pattern length
   if pattern.len() > 100 {
       return Err(Error::pattern_too_long());
   }
   
   // Use regex::RegexBuilder with size limits
   let regex = RegexBuilder::new(pattern)
       .size_limit(1000)  // Default is 10MB
       .dfa_size_limit(1000)  // Limit DFA compilation
       .build()?;
   ```

### For MEDIUM Severity Patterns (Configuration)

1. **Add profile configuration validation**
   - Validate patterns when loading profiles
   - Reject patterns with nested quantifiers
   - Limit pattern complexity

2. **Implement pattern sandboxing**
   - Run complex patterns with timeouts
   - Monitor regex execution time
   - Fall back to safe patterns on timeout

### General Recommendations

1. **Add DoS protection**: Time limits on all regex operations
2. **Monitoring**: Log slow regex operations (>100ms)
3. **Testing**: Add fuzz testing for regex input handling
4. **Documentation**: Document safe patterns and input requirements

## Compliance and Security Impact

- **OWASP Regex DoS:** These vulnerabilities directly map to OWASP's Regular Expression Denial of Service (ReDoS) category
- **CWE:** CWE-1333 - Inefficient Regular Expression Complexity
- **CVSS Impact:** Potential CVSS score of 7.5 (HIGH) for CRITICAL patterns due to:
  - Network exposure (MCP interface)
  - Low attack complexity  
  - High availability impact (DoS)

## Testing Recommendations

1. **Add integration tests** for malicious pattern inputs
2. **Fuzz testing** on user-input regex paths
3. **Performance benchmarking** to detect slow patterns
4. **Regression tests** for known bad patterns

## References

- OWASP Regular Expression Denial of Service (ReDoS)
- Rust regex crate safety documentation  
- CWE-1333: Inefficient Regular Expression Complexity
- "Regular Expression Denial of Service (ReDoS)" - OWASP
- Rust regex crate: https://docs.rs/regex/latest/regex/

## Conclusion

The pdftract codebase has **6 regex patterns** at risk of catastrophic backtracking vulnerabilities, with **3 at CRITICAL severity** that accept direct user input without proper validation. The most critical issues are in the search functionality and MCP tool interface, which could be exploited for DoS attacks. 

**Immediate action required** for the 3 CRITICAL patterns to prevent potential denial-of-service attacks through malicious regex input.
