> **Correction (2026-09-23):** The earlier hash-mismatch finding was computed
> against the stale expected value `bd9a3533...` and is withdrawn. The
> authoritative provenance record cited by the task at
> `tests/fixtures/PROVENANCE.md:301` records the expected SHA-256 as
> `2eaeba307f622ca6b6d6174bd1bdd9a1310268c7d4f101d3e2a222ba8b719689` (the
> `fingerprint-match.pdf` entry is currently at lines 338-346), and the
> on-disk file matches it byte for byte. There is no fixture-corruption
> finding.

# bf-3bnao step 4: Verify pdftract completed successfully

**Date:** 2026-09-23
**Fixture:** `tests/fixtures/encoding/fingerprint-match.pdf`
**State:** PASS

## Verification command

The current CLI uses the `extract` subcommand. From a clean `git archive HEAD`
extraction, with the execution target isolated from the shared checkout, I ran:

```sh
unset RUST_LOG
cargo run -- extract tests/fixtures/encoding/fingerprint-match.pdf --json -
```

`RUST_LOG` was unset before invocation. The command exited with **0**.

### Captured stdout

The command wrote 1,143 bytes of JSON to stdout. Sample:

```json
{
  "attachments": [],
  "fingerprint": "pdftract-v1:ab24a95f44ceca5d2aed4b6d056adddd8539f44c6cd6ca506534e830c82ea8a8",
  "form_fields": [],
  "metadata": {
    "block_count": 1,
    "cache_status": "skipped",
    "page_count": 1,
    "span_count": 1
  },
  "pages": [
    { "index": 0, "blocks": [{ "kind": "paragraph", "text": "Test" }] }
  ],
  "schema_version": "1.0"
}
```

`jq -e .` accepted the complete captured stdout (exit 0).

### Captured stderr

stderr was empty (0 bytes). No extraction error was emitted.

## Parent acceptance criteria

These are the four criteria from `bf-1aow8`:

1. **PASS** — Exit code from the command is 0: observed exit code **0**.
2. **PASS** — Output contains valid JSON or the expected pdftract format: the
   1,143-byte output parses successfully with `jq -e .` (exit 0), contains a
   one-page document, and contains the extracted text `Test`.
3. **PASS** — No error messages in stderr: captured stderr is empty.
4. **PASS** — Document verification results are recorded in this file.

The previous failed step-3 record is superseded by this clean committed-HEAD
rerun. For the root-cause analysis of the earlier failure path, see
`notes/bf-1aow8-diagnosis.md`; this note records the verification result and
does not re-derive that analysis.

## Fixture and companion-file checks

The on-disk fixture hash is:

```text
2eaeba307f622ca6b6d6174bd1bdd9a1310268c7d4f101d3e2a222ba8b719689  tests/fixtures/encoding/fingerprint-match.pdf
```

This matches the expected value in `tests/fixtures/PROVENANCE.md` exactly.

The pre-existing untracked file
`tests/fixtures/encoding/fingerprint-match.txt` was inspected and is 4 bytes
with content `Test` (created 2026-09-05). It is the companion expected-content
text file, not part of this verification change. It remains untracked and was
not included in the commit.
