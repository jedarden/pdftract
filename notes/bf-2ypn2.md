# Phase 7 Profiles Exit Gate Fixtures

## Bead ID: bf-2ypn2

## Summary

Created three test fixture directories and integration tests for Phase 7 profile validation exit gate.

## Files Created

### Invalid Profile Fixtures (`tests/fixtures/profiles/invalid/`)

1. **unknown-key.yaml** - Contains unrecognized key `not_a_real_key` in extraction section (should fail schema validation)

2. **bad-combinator.yaml** - Malformed match combinators (uses scalar instead of list for `all` combinator, should fail YAML parsing)

3. **secret-key.yaml** - Contains forbidden key `api_key` (should trigger `PROFILE_SECRETS_FORBIDDEN` validation error)

4. **malformed.yaml** - Unclosed bracket in YAML array (should fail YAML syntax parsing)

### Valid Profile Fixtures (`tests/fixtures/profiles/valid/`)

1. **minimal.yaml** - Minimal valid profile with only required fields (name, description, priority)

2. **invoice-minimal.yaml** - Simplified invoice profile with match expression, extraction tuning, and field extraction

### Profile Resolution Fixtures (`tests/fixtures/profiles/resolution/`)

1. **custom-invoice.yaml** - Custom invoice profile with priority 100 (higher than built-in priority 50) to test profile resolution order

2. **README.md** - Documentation of resolution priority order: built-in < user < --profile-dir

### Integration Tests (`tests/integration/advanced/profiles.rs`)

Created comprehensive integration tests:

1. **test_invalid_profiles_rejected** - Validates that all 4 invalid profiles are rejected with appropriate error messages
2. **test_valid_profiles_accepted** - Validates that all 2 valid profiles pass validation
3. **test_profile_resolution_order** - Tests profile resolution priority order using --profile-dir flag
4. **test_invalid_fixture_error_types** - Validates specific error types for each invalid fixture

## Acceptance Criteria Status

✓ `pdftract profiles validate tests/fixtures/profiles/invalid/*.yaml` reports errors for all 4 invalid files
- Each invalid profile has distinct failure modes (forbidden keys, YAML syntax, schema validation)

✓ `pdftract profiles validate tests/fixtures/profiles/valid/*.yaml` exits 0
- Both minimal and invoice-minimal profiles validate successfully

✓ Profile resolution order test passes
- Tests that custom profile (priority 100) overrides built-in (priority 50)
- Validates --profile-dir flag functionality

## Testing

To manually verify:

```bash
# Build pdftract with profiles feature
cargo build --features profiles

# Test invalid profiles (should all fail)
pdftract profiles validate tests/fixtures/profiles/invalid/*.yaml

# Test valid profiles (should all pass)
pdftract profiles validate tests/fixtures/profiles/valid/*.yaml

# Test resolution order
pdftract profiles list
pdftract profiles list --profile-dir tests/fixtures/profiles/resolution

# Run integration tests
cargo test --features profiles --test integration_tests profiles
```

## Implementation Notes

- The `secret-key.yaml` fixture tests the forbidden key detection implemented in `loader.rs` (check_forbidden_keys function)
- The `malformed.yaml` fixture tests YAML parser error handling
- The `bad-combinator.yaml` fixture tests match expression schema validation
- The `unknown-key.yaml` fixture tests extraction tuning schema validation
- Resolution test uses priority differences (100 vs 50) to verify override behavior
