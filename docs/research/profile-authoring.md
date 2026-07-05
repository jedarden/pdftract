# Profile Authoring Guide — YAML Footguns and Validation

This guide documents YAML authoring gotchas for document profiles and the `pdftract profiles validate` command. Required by [ADR-007](../plan/plan.md#adr-007-use-yaml-not-toml-or-json-for-profile-templates).

## 1. YAML Footguns

### 1.1 Scalar Type Coercion

YAML implicitly converts certain unquoted strings to boolean or integer values. **Always quote strings** that could be misinterpreted:

```yaml
# ❌ WRONG — YAML parses "on" as boolean true
enabled: on
yes_count: yes
no_count: no
true_value: true

# ✅ CORRECT — quoted strings preserve literal values
enabled: "on"
yes_count: "yes"
no_count: "no"
true_value: "true"
```

**Common coercion patterns that bite profile authors:**

| Literal string | YAML interprets as | Quote to preserve |
|----------------|-------------------|-------------------|
| `on`, `On`, `ON` | `true` (boolean) | `"on"` |
| `off`, `Off`, `OFF` | `false` (boolean) | `"off"` |
| `yes`, `Yes`, `YES` | `true` (boolean) | `"yes"` |
| `no`, `No`, `NO` | `false` (boolean) | `"no"` |
| `true`, `True`, `TRUE` | `true` (boolean) | `"true"` |
| `false`, `False`, `FALSE` | `false` (boolean) | `"false"` |
| `123`, `3.14` | integer/float | `"123"` |

### 1.2 Significant Indentation

YAML is indentation-sensitive (like Python). **Use spaces, not tabs**. Consistent 2-space indentation is recommended for profiles:

```yaml
# ❌ WRONG — inconsistent indentation breaks YAML parsing
match:
all:
  - any:
      - text_contains:
          patterns: ["invoice"]

# ✅ CORRECT — consistent 2-space indentation
match:
  all:
    - any:
        - text_contains:
            patterns: ["invoice"]
```

### 1.3 Multi-line Strings

For regex patterns with quotes or complex strings, use proper multi-line syntax:

```yaml
# ❌ WRONG — unescaped special characters break parsing
patterns: ["invoice["]  # Unclosed bracket
patterns: ["invoice\"total\""]  # Confusing quote escaping

# ✅ CORRECT — use YAML block scalar for complex strings
patterns: [
  "invoice\\[",
  "invoice\\[.*\\]",
  "invoice\\s*#\\s*([\\w-]+)"
]
```

**Block scalar syntax for very long patterns:**

```yaml
# Literal block scalar (|-) preserves newlines, no escaping
long_regex: |-
  Invoice\s*#\s*[\w-]+
  .*
  Total:\s*\$?\s*[\d,]+\.\d{2}

# Folded block scalar (>) folds newlines into spaces
description: >
  This profile matches commercial invoices
  with line items, vendor/customer fields,
  and monetary totals.
```

## 2. The `match:` Combinator DSL

The `match:` section uses boolean combinators to define document classification rules. Three combinators are available: `all:`, `any:`, and `none:`.

### 2.1 Basic Structure

```yaml
match:
  all:                    # ALL conditions must be satisfied
    - any:                # At least ONE condition must be satisfied
        - text_contains:
            patterns: ["invoice"]
        - heading_matches:
            pattern: "^Invoice\\b"
    - structural:         # AND this structural condition
        has_table: true
        
  none:                   # NONE of these conditions may match
    - text_contains:
        patterns: ["abstract", "scientific paper"]
```

### 2.2 Common Mistakes

**❌ WRONG — Single value instead of list under `all:`:**

```yaml
match:
  all:
    text_contains:        # Invalid: all expects a list
      patterns: ["invoice"]
```

**✅ CORRECT — List structure for combinators:**

```yaml
match:
  all:
    - text_contains:      # Valid: list of predicates
        patterns: ["invoice"]
```

**❌ WRONG — Mixing `any:` with direct conditions:**

```yaml
match:
  any:
    - text_contains:
        patterns: ["invoice"]
    structural:           # Invalid: not in a list item
      has_table: true
```

**✅ CORRECT — All conditions inside list items:**

```yaml
match:
  any:
    - text_contains:
        patterns: ["invoice"]
    - structural:
        has_table: true
```

### 2.3 Predicate Types

#### `text_contains`
Searches page text for pattern matches:

```yaml
- text_contains:
    patterns: ["invoice", "bill to", "invoice #"]
    case_sensitive: false  # optional, default: false
    min_hits: 1           # optional, default: 1
```

#### `heading_matches`
Matches against heading text:

```yaml
- heading_matches:
    pattern: "^Invoice\\b"  # Regex anchored to heading start
    case_sensitive: false   # optional
```

#### `structural`
Tests document structure:

```yaml
- structural:
    has_table: true
    has_form_field: false
    has_math: false
    page_count:
      min: 1
      max: 5
```

## 3. The `fields:` DSL

The `fields:` section defines extracted fields and their extraction strategies.

### 3.1 Field Definition Structure

```yaml
fields:
  invoice_number:
    type: string              # Field type: string, date, decimal, int, bool, array
    extraction:
      regex: "Invoice\\s*#\\s*([\\w-]+)"    # Capture group 1 is the value
      near: ["Invoice", "Invoice #"]         # Search near anchor text
      max_distance_pt: 200                   # Max distance in points
      parse: string                          # Parser: string, decimal, date, int, bool
      region: top_quarter                    # Optional region hint
      pick: nearest_below                    # Disambiguation strategy
```

### 3.2 Field Extraction Strategies

#### `regex:` Pattern Matching

```yaml
# ❌ WRONG — Unescaped regex special characters
regex: "Invoice # ([\w-]+)"     # Space is ambiguous

# ✅ CORRECT — Proper escaping
regex: "Invoice\\s*#\\s*([\\w-]+)"
```

#### `near:` Anchor Text Search

```yaml
# Search near anchor text (list of alternatives)
near: ["Invoice", "Invoice #", "Invoice Number"]
max_distance_pt: 200
```

#### `region:` Page Region Hints

```yaml
# Restrict search to page regions
region: top_quarter      # | bottom_quarter | left_half | right_half
region: top:50           # Top 50 points
region: bbox:[0,0,200,100]  # Explicit bounding box [x0,y0,x1,y1]
```

#### `pick:` Disambiguation

```yaml
# When multiple candidates match, pick one
pick: largest_font       # | smallest_font | nearest_below | nearest_right | first | last
```

#### `parse:` Type Coercion

```yaml
# Parse matched text into typed values
parse: string            # | decimal | date | int | bool

# Date parsing is heuristic (tries common formats)
parse: date              # Detects: 2025-09-14, 09/14/2025, 14-Sep-2025, etc.
```

### 3.3 Common `fields:` Mistakes

**❌ WRONG — Missing extraction block:**

```yaml
fields:
  invoice_number:
    type: string
    regex: "Invoice\\s*#\\s*([\\w-]+)"  # Invalid: regex must be under extraction
```

**✅ CORRECT — Proper extraction nesting:**

```yaml
fields:
  invoice_number:
    type: string
    extraction:
      regex: "Invoice\\s*#\\s*([\\w-]+)"
```

**❌ WRONG — Invalid parse type combination:**

```yaml
fields:
  total:
    type: decimal
    extraction:
      parse: bool        # Inconsistent: type is decimal but parse is bool
```

**✅ CORRECT — Consistent types:**

```yaml
fields:
  total:
    type: decimal
    extraction:
      parse: decimal
```

## 4. Common Validation Errors

The `pdftract profiles validate` command catches common mistakes. Run it after editing any profile:

```bash
pdftract profiles validate my-profile.yaml
```

### 4.1 YAML Parse Errors

**Error message:**
```
Error: YAML parse error at line 9, column 10:
  unclosed bracket '['
```

**Causes:**
- Unclosed brackets: `patterns: ["test"`
- Unescaped quotes: `patterns: ["invoice["]`
- Inconsistent indentation
- Tab characters (use spaces)

### 4.2 Schema Validation Errors

**Error message:**
```
Error: Profile validation failed:
  - Unknown key 'not_a_real_key' at extraction.not_a_real_key
  - Missing required key 'name' at profile root
```

**Causes:**
- Typos in key names
- Missing required fields
- Keys not recognized by the profile schema

### 4.3 PROFILE_SECRETS_FORBIDDEN Errors

**Error message:**
```
Error: PROFILE_SECRETS_FORBIDDEN — profiles must not contain secret keys:
  Forbidden key 'api_key' found at extraction.api_key
  Forbidden key 'password' found at fields.login.extraction
```

**Causes:**
- Including secret keys in profiles (forbidden by security policy)
- Common forbidden keys: `api_key`, `password`, `token`, `secret`, `credential`

**Why this is forbidden:** Profiles are configuration files that may be checked into version control or shared. Secrets must be injected via environment variables, not embedded in profiles.

### 4.4 Combinator Structure Errors

**Error message:**
```
Error: Invalid combinator structure:
  'all' requires a list, found single value at match.all
```

**Causes:**
- Single value instead of list under `all:`, `any:`, `none:`
- Mixing list items with direct key-value pairs

## 5. Profile Search Path Resolution

Profiles are resolved from multiple sources in order of priority (later sources override earlier ones on name collision):

1. **Built-in profiles** (compiled into binary at `profiles/builtin/`)
2. **System-wide profiles** (`/etc/pdftract/profiles/*.yaml`)
3. **User profiles** (`$XDG_CONFIG_HOME/pdftract/profiles/*.yaml`, defaults to `~/.config/pdftract/profiles/`)
4. **Runtime directories** (`--profile-dir DIR` CLI flag, repeatable)

### 5.1 Shadowing Behavior

A user profile shadows a built-in profile of the same name:

```bash
# Export built-in invoice profile
pdftract profiles export invoice > ~/.config/pdftract/profiles/invoice.yaml

# Edit the file...
vim ~/.config/pdftract/profiles/invoice.yaml

# Next invocation uses the modified version
pdftract extract --profile invoice file.pdf
```

Verify which profile is active:

```bash
pdftract profiles list
# Output:
# invoice (user, overrides built-in)
# receipt (built-in)
# contract (built-in)
```

### 5.2 Resolution Order Example

```bash
# Assume built-in profile 'invoice' exists
# User creates ~/.config/pdftract/profiles/invoice.yaml
# System has /etc/pdftract/profiles/invoice.yaml

pdftract extract --profile invoice file.pdf
# Uses ~/.config/pdftract/profiles/invoice.yaml (highest priority)

pdftract extract --profile-dir /tmp/custom_profiles --profile invoice file.pdf
# Uses /tmp/custom_profiles/invoice.yaml (CLI flag highest priority)
```

## 6. PROFILE_SECRETS_FORBIDDEN Constraint

### 6.1 What Is Forbidden

Profiles must not contain secret keys or sensitive credentials. The following key patterns are rejected at validation:

- `api_key`
- `password`
- `token`
- `secret`
- `credential`
- `private_key`
- `auth_token`

### 6.2 Why This Exists

Profiles are configuration files intended to be:
- Checked into version control
- Shared across teams
- Documented in examples and tutorials

Embedding secrets in profiles violates the principle that **configuration is code, secrets are environment**.

### 6.3 Correct Pattern: Environment Variables

If you need to parameterize a profile, use environment variable references instead of hardcoded secrets:

```yaml
# ❌ WRONG — Hardcoded secret
extraction:
  api_key: "[REDACTED]"

# ✅ CORRECT — Environment variable reference (implementation-specific)
# Profiles should avoid needing secrets entirely; pass credentials via CLI/env
```

**Note:** The current profile design does not support environment variable interpolation. If your use case requires secrets, it may not be a good fit for profile-based extraction.

## 7. Complete Annotated Example

Here's a complete, valid invoice profile with annotations:

```yaml
# Invoice extraction profile
# Matches commercial invoices with line items, vendor/customer, and totals
name: invoice
description: Commercial invoice with line items, vendor/customer, and totals
priority: 50

# Document classification: matches invoices only if ALL conditions are satisfied
match:
  all:
    # At least one heading/heading pattern must match
    - any:
        - text_contains:
            patterns: ["invoice", "bill to", "invoice #", "invoice number", "tax invoice"]
            case_sensitive: false
        - heading_matches:
            pattern: "^Invoice\\b"
    
    # Must have currency pattern OR a table
    - any:
        - has_currency_pattern:
            has_currency_pattern: true
        - structural:
            has_table: true
            has_form_field: false
            page_count:
              min: 1
              max: 5
  
  # Must NOT contain these academic paper markers
  none:
    - text_contains:
        patterns: ["abstract", "bibliography", "scientific paper"]

# Extraction tuning parameters
extraction:
  reading_order: line_dominant      # Use line-based reading order
  table_detection: strict_borders   # Strict table boundary detection
  readability_threshold: 0.4        # Skip text blocks below readability threshold
  include_invisible: false          # Exclude invisible text
  include_headers_footers: false    # Exclude headers/footers
  force_ocr: false                  # Don't force OCR if text exists
  min_block_chars: 0               # No minimum character threshold

# Field extraction definitions
fields:
  invoice_number:
    type: string
    extraction:
      regex: "Invoice\\s*#\\s*([\\w-]+)"
      near: ["Invoice", "Invoice Number", "Invoice #"]
      max_distance_pt: 200
      parse: string
  
  vendor:
    type: string
    extraction:
      region: top_quarter
      pick: largest_font
  
  customer:
    type: string
    extraction:
      near: ["Bill To", "Customer", "Sold To"]
      max_distance_pt: 150
      pick: nearest_below
      parse: string
  
  invoice_date:
    type: date
    extraction:
      near: ["Date", "Invoice Date"]
      max_distance_pt: 100
      parse: date
  
  total:
    type: decimal
    extraction:
      regex: "([\\d,]+\\.\\d{2})"
      near: ["Total", "Amount Due", "Balance Due", "Grand Total"]
      max_distance_pt: 80
      parse: decimal
  
  line_items:
    type: array
    extraction:
      table_region: largest_table
      schema:
        - name: description
          type: string
          required: true
        - name: quantity
          type: decimal
          required: false
        - name: unit_price
          type: decimal
          required: false
        - name: amount
          type: decimal
          required: false
      fallback: []    # Empty array if no table found
```

## 8. Validation Workflow

Recommended workflow when authoring profiles:

1. **Create the profile:**
   ```bash
   vim ~/.config/pdftract/profiles/my-profile.yaml
   ```

2. **Validate syntax:**
   ```bash
   pdftract profiles validate ~/.config/pdftract/profiles/my-profile.yaml
   ```

3. **Test against fixture:**
   ```bash
   pdftract extract --profile my-profile fixture.pdf
   ```

4. **Iterate:** Edit and re-validate until validation passes

5. **Check for shadowing:**
   ```bash
   pdftract profiles list | grep my-profile
   ```

6. **Commit and test:** If sharing with a team, commit to version control and validate in CI

## References

- [ADR-007: Use YAML for profile templates](../plan/plan.md#adr-007-use-yaml-not-toml-or-json-for-profile-templates) — Rationale for YAML over TOML/JSON
- [Phase 7.10: Document Profiles and Field Extraction](../plan/plan.md#710-document-profiles-and-field-extraction) — Full profile specification
- [Profile validation tests](../../tests/integration/advanced/profiles.rs) — Integration test suite
- [Example profiles](../../profiles/builtin/) — Built-in profile implementations
