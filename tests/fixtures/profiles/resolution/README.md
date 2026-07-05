# Profile Resolution Test Fixtures

This directory tests the profile resolution priority order:

**Priority order (lowest to highest):**
1. Built-in profiles (compiled into binary)
2. System profiles (`/etc/pdftract/profiles/`)
3. User profiles (`~/.config/pdftract/profiles/`)
4. Custom profiles (`--profile-dir` flag, repeatable)

**Test scenarios:**

- `custom-invoice.yaml` - A custom invoice profile with priority 100 that overrides the built-in invoice profile (priority 50)
- Expected: When loaded via `--profile-dir`, the custom profile should take precedence over the built-in one
- Verification: Run `pdftract profiles list` with and without `--profile-dir` to see the override annotation

**Test procedure:**

```bash
# List built-in profiles (should show invoice with priority 50)
pdftract profiles list

# List with custom directory (should show invoice with priority 100 and overrides annotation)
pdftract profiles list --profile-dir tests/fixtures/profiles/resolution

# Validate the custom profile
pdftract profiles validate tests/fixtures/profiles/resolution/custom-invoice.yaml
```
