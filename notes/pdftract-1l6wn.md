# Verification Note: pdftract-1l6wn - BMC/BDC/EMC Operator Parsers

## Bead Description

Implement the 3 marked-content operators: `BMC /Tag`, `BDC /Tag <<props>>` or `BDC /Tag /PropName`, and `EMC`. Maintain a `marked_content_stack: Vec<MarkedContentFrame>` on the page executor.

## Status: ALREADY IMPLEMENTED ✅

The BMC/BDC/EMC operator parsers and `MarkedContentStack` are already fully implemented in the codebase.

## Implementation Location

### 1. MarkedContentStack
- **File**: `crates/pdftract-core/src/parser/marked_content_stack.rs`
- **Structure**: `MarkedContentStack` with `Vec<MarkedContentFrame>`
- **Depth limit**: 64 (enforced with `MAX_MC_DEPTH` constant)
- **Diagnostics**: `EmcWithoutBmc`, `MarkedContentDepthExceeded`

### 2. MarkedContent Operators
- **File**: `crates/pdftract-core/src/parser/marked_content_operators.rs`
- **Functions**:
  - `parse_bmc(stack, tag)` - BMC operator
  - `parse_bdc(stack, tag, props, resources, default_off_ocgs, diagnostics)` - BDC operator
  - `parse_emc(stack)` - EMC operator

### 3. Integration into Content Stream
- **File**: `crates/pdftract-core/src/content_stream.rs`
- **Function**: `execute_with_do` (lines 875-965)
- The marked-content stack is instantiated and BMC/BDC/EMC operators are wired into the main content stream execution loop.

## Acceptance Criteria Verification

All acceptance criteria are covered by passing tests:

| Criterion | Test Name | Status |
|-----------|-----------|--------|
| BMC /Span EMC: stack push then pop, balanced | `test_push_bmc` + `test_pop_emc` | ✅ PASS |
| BDC /Span <</MCID 42>> EMC: frame with mcid=Some(42) | `test_parse_bdc_with_inline_dict_mcid` | ✅ PASS |
| BDC /P /MyProps EMC with resolved /MCID 7 | `test_parse_bdc_with_property_name_found` | ✅ PASS |
| BDC /P /UnknownProps EMC: diagnostic for unknown prop name | `test_parse_bdc_with_property_name_not_found` | ✅ PASS |
| EMC without BMC: EMC_WITHOUT_BMC diagnostic, no panic | `test_pop_emc_underflow` | ✅ PASS |
| 64 nested BMC: succeeds; 65th emits depth diagnostic | `test_stack_depth_limit` | ✅ PASS |

## Test Results

```
cargo test -p pdftract-core --lib 2>&1 | grep -E "test.*marked_content|test.*parse_bmc|test.*parse_bdc|test.*parse_emc"
```

All 57 marked-content related tests passed:
- 18 tests in `parser::marked_content_stack::tests`
- 30 tests in `parser::marked_content_operators::tests`
- 9 tests in other modules using marked-content features

## Key Implementation Details

### INV: Marked Content Stack Independence
The marked-content stack is independent of the graphics state stack. The q/Q operators do NOT affect it (per PDF spec section 14.5). This is correctly implemented in `content_stream.rs` where the two stacks are managed separately.

### Property Name Resolution
`ResourceDict::lookup_properties()` is used to resolve property names in BDC operators. The implementation handles both inline dictionaries and property resource names.

### MCID Extraction
- Supports Integer and Real (whole number) MCID values
- Negative MCID values are rejected per spec ("non-negative integer")
- Missing /MCID results in `mcid=None` (valid for BMC or BDC without properties)

### OCG Hidden Flag (bead pdftract-1q19p)
The `is_hidden` flag on `MarkedContentFrame` tracks content within default-OFF Optional Content Groups. This is integrated into the BDC parser for /OC tags.

## Conclusion

No implementation work was required for this bead. The BMC/BDC/EMC operator parsers and `MarkedContentStack` were already fully implemented with comprehensive test coverage. All acceptance criteria are met.
