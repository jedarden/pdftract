# pdftract-908fe7ff — `extract_content_stream_bytes`: error contract and pinned tests

Pre-change inventory behind the pdftract-843a369b umbrella (bf-2yncbh). This is
the written account the validation children (pdftract-7a42320d,
pdftract-0914d4cd, pdftract-0669ada9) were asked to start from, so that an
error or a decode nothing depends on does not change silently.

**Scope: documentation only. This bead's diff touches no `.rs` files.**

Snapshot: working tree of HEAD `0b3e6f16`, 2026-09-07. All line numbers below
are from that snapshot — the file is shared with in-flight sibling work, so
re-grep before trusting a number (see §0).

---

## 0. Snapshot caveats — read before using any line number

Three files this note describes are **dirty in the shared checkout right now**,
edited by sibling workers on the same umbrella:

| File | Dirty state at snapshot | Effect on this note |
|---|---|---|
| `crates/pdftract-core/src/font/type3_rasterizer.rs` | 2 hunks, at ~2410 and ~6652 | Neither touches the function (2222–2315) or the validator (388–490); numbers here are valid for HEAD too |
| `crates/pdftract-core/tests/type3_charproc_structure.rs` | **+368 lines** | The entire "Resolution-path coverage" half (lines ~171–500) is **uncommitted** sibling work; the committed file stops at ~168 |
| `crates/pdftract-core/src/extract.rs` | uncommitted signature change | Breaks the crate build (see §7) |

Practical consequence: `tests/type3_charproc_structure.rs` line numbers shift by
~+19 once the in-flight half lands, and half of that file does not exist at
HEAD. Inventory below describes the **worktree** state, since that is what the
next worker will see.

---

## 1. Per-arm error contract

`pub fn extract_content_stream_bytes(resolved_obj: PdfObject, doc_context:
&DocumentContext) -> Result<Vec<u8>, Type3Error>` at
`crates/pdftract-core/src/font/type3_rasterizer.rs:2222`. Signature is **by
value** on `resolved_obj` — the caller's `PdfObject` is consumed (contrast the
validator, which takes `&PdfObject`). The `Ref` arm is the only arm that
consumes that fact (it moves the inner object into the recursive call).

| Line | Arm | Behavior today |
|---|---|---|
| 2239 | `Stream(stream)` | Decodes and returns `Ok(bytes)`. Uses `ExtractionOptions::default()` and `decode_stream(&stream, source, &opts, &mut 0u64)` (`parser/stream.rs:3670`). **This arm cannot fail.** `decode_stream` returns `Vec<u8>`, "or an empty Vec if decoding failed completely" — an undecodable stream yields `Ok(vec![])`, never an `Err`. Also note it calls plain `decode_stream`, not `decode_stream_with_decryption`, so an encrypted charproc decodes to garbage/empty rather than erroring. |
| 2248 | `Ref(obj_ref)` | `resolver.resolve_with_source(obj_ref, source)` then **recurses** into `extract_content_stream_bytes(inner_obj, doc_context)` (line 2255). Resolve errors map through `From<ResolveError> for Type3Error` (`NotFound` → `MissingCharProcRef`, `CircularRef` → `CircularRef`, `Io` → `Io`); that mapping is the *only* source of non-`Io` errors in the whole function today. Cycle safety comes from the resolver's `ResolutionGuard`, not from any local depth counter — there is no `MAX_GLYPH_DEPTH`-style bound on this recursion, and a `Ref` whose target is another `Ref` recurses again (parser-produced objects can nest `Ref`s, so the chain is reachable). |
| 2257 | `Null` | `Type3Error::Io("Cannot extract stream from null object at char_proc_ref")` — **note: no `- expected Stream or Ref` suffix**, unlike every other wrong-type arm. |
| 2262 | `Bool(_)` | `Io("Cannot extract stream from boolean at char_proc_ref - expected Stream or Ref")` |
| 2267 | `Integer(_)` | `Io("Cannot extract stream from integer at char_proc_ref - expected Stream or Ref")` |
| 2272 | `Real(_)` | `Io("Cannot extract stream from real number at char_proc_ref - expected Stream or Ref")` |
| 2277 | `String(_)` | `Io("Cannot extract stream from string at char_proc_ref - expected Stream or Ref")` |
| 2282 | `Name(_)` | `Io("Cannot extract stream from name at char_proc_ref - expected Stream or Ref")` |
| 2287 | `Array(_)` | `Io("Cannot extract stream from array at char_proc_ref - expected Stream or Ref")` |
| 2292 | `Dict(_)` | `Io("Cannot extract stream from dictionary at char_proc_ref - expected Stream or Ref")` — see §4; this is the arm the Dict decision is about. |
| 2297 | `Indirect(_)` | `Io("Cannot extract stream from indirect object at char_proc_ref - expected Stream or Ref")` — see the wrinkle in §4. |

The type words in those strings are **hand-written**, not `PdfObject::type_name()`
(`parser/object/types.rs:258`). The enum's own names differ in two places:
`Bool` → `"boolean"` and `Real` → `"real"` — so `Integer(5)` reports
`"integer"` either way, but a reworded error built from `type_name()` must use
`"boolean"`, not `"bool"`. `Null` → `"null"`, `Dict` → `"dictionary"`,
`Indirect` → `"indirect"`, `Ref` → `"reference"`.

The doc comment above the function (2199–2221) still advertises only
"wrong type / I/O error / invalid reference" and does not mention that
wrong types are `Io` — pdftract-7a42320d owns adding an `# Errors` section.

### Reachability after entry validation (pdftract-7a42320d)

Once `validate_char_proc_structure(&resolved_obj)` runs on entry:

- `Null`, `Bool`, `Integer`, `Real`, `String`, `Name`, `Array` arms become
  **unreachable dead code** (validation rejects them first with
  `InvalidCharProcType`).
- `Dict(_)` stays **reachable and must keep erroring** — validation *accepts*
  a dict (§4).
- `Indirect(_)` becomes a **trap**, not dead code — see §4.
- `Stream` stays reachable, but a stream missing `/Type`, `/Subtype`, `/Width`
  or `/Height` now errors as `MissingRequiredKey` before any decode happens.
- `Ref` stays reachable; the recursive call re-validates the inner object
  automatically because it re-enters the function.

---

## 2. Precondition order — the two `Io` checks run first

Lines 2226–2236, before the `match` at 2238:

1. `doc_context.resolver` is `None` → `Io("XrefResolver not provided in
   DocumentContext - cannot extract stream")`
2. `doc_context.source` is `None` → `Io("PdfSource not provided in
   DocumentContext - cannot extract stream")`

**Any entry validation must be inserted after these two checks, not before
them.** The pinned unit test
`test_extract_content_stream_bytes_without_resolver_returns_type3_error`
(line 2975) feeds a `PdfObject::Dict` into a context with `resolver: None,
source: None` and asserts the **precondition** `Io` — if validation ran first,
that test would see `MissingRequiredKey` (or `InvalidCharProcType`) instead and
fail. pdftract-7a42320d's criterion 3 requires that test to pass **with no
edits**.

The same ordering already exists in `deref_char_proc_ref` (2143): context
missing (2145) → resolver missing (2153) → source missing (2161) → resolve
(2171) → `validate_char_proc_structure` (2176). Validation is last there too.

---

## 3. Every test that pins this behavior

### 3a. In-file unit tests — `crates/pdftract-core/src/font/type3_rasterizer.rs` (`#[cfg(test)]` mod)

| Line | Test | What it pins |
|---|---|---|
| **2975** | `test_extract_content_stream_bytes_without_resolver_returns_type3_error` | **The one that touches `extract_content_stream_bytes` directly.** `Dict` input + `resolver: None` → `Type3Error::Io` containing `"XrefResolver not provided"`. Pins precondition-first ordering; must survive entry validation unedited. |
| 2722 | `test_deref_char_proc_ref_without_context_returns_error` | `deref` with `None` context → `Io` "DocumentContext not provided" |
| 2738 | `test_deref_char_proc_ref_without_resolver_returns_error` | `Io` "XrefResolver not provided" from `deref` |
| 2759 | `test_deref_char_proc_ref_without_source_returns_error` | `Io` "PdfSource not provided" from `deref` |
| 2782–2866 | `test_type3_error_*` (6 tests) | `Type3Error` `Display` strings and the `From<ResolveError>` mapping (`NotFound`→`MissingCharProcRef`, `CircularRef`→`CircularRef`, `Io`→`Io`) — the vocabulary the `Ref` arm's errors come out in |
| 2880 | `test_deref_char_proc_ref_validates_structure_before_returning` | Empty stream dict → `MissingRequiredKey` (key contains `/Type` or `/Width`); EC-42 early-validation comment |
| 2906 | `test_deref_char_proc_ref_validation_includes_ref_context` | `Integer` → `InvalidCharProcType`; error text mentions the type |
| 2928 | `test_deref_char_proc_ref_passes_valid_stream` | Stream with `/Type /XObject /Subtype /Form /Width /Height` → validation `Ok`, `detect_char_proc_type` → `Stream`. **Fixture uses slashed keys** (`intern("/Type")`) — see §5.2: this passes today only because the validator's lookups are slashed too. |
| 2952 | `test_detect_char_proc_type_returns_unknown_for_failed_deref` | `Ref` + no context → `CharProcType::Unknown` (bf-5on6og, §5.3) |
| 2996 / 3020 / 3063 | `test_rasterize_type3_glyph_with_{missing_glyph,failed_resolution,malformed_stream}_returns_none` | Adjacent graceful-degradation contract: bytes arrive via the `StreamResolverFn` callback instead, and bad bytes degrade to `None`, never panic. These do **not** go through `extract_content_stream_bytes`. |
| 3351 | `test_resolve_stream_callback_receives_parameters` | Callback receives the `ObjRef`; the doc comment points at `resolver.rs` ~700 as the production pattern |

### 3b. Integration tests — `crates/pdftract-core/tests/type3_charproc_structure.rs`

**Committed half (validator-direct, `validate_char_proc_structure`):**

| Line (worktree) | Test | Pins |
|---|---|---|
| 54 | `valid_stream_is_accepted` | Complete stream → `Ok(())` |
| 62 | `filter_is_optional_on_stream_dicts` | `/Filter` is not required |
| 69 | `stream_missing_subtype_is_rejected` | `MissingRequiredKey { "/Subtype", "stream" }` |
| 83 | `valid_dict_is_accepted` | `/Type`+`/Subtype` dict → `Ok(())` — **the dict-acceptance rule behind §4** |
| 95 | `dict_missing_type_is_rejected` | `MissingRequiredKey { "/Type", "dictionary" }` |
| 109 | `scalar_types_are_rejected_with_their_own_name` | Integer/Real/Null → `InvalidCharProcType { got, expected: "stream or dictionary" }` |
| 128 | `unresolved_reference_is_reported_as_unknown` | `Ref` → `InvalidCharProcType { got: "unknown" }` (bf-5on6og) |
| 141 | `indirect_stream_is_accepted` | `Indirect` wrapper unwrapped, valid stream → `Ok(())` |
| 151 | `indirect_stream_still_checks_required_keys` | Keys checked through the wrapper |
| 165 | `indirect_scalar_reports_the_carried_type` | `Indirect(Integer)` → `got: "integer"` |

**In-flight half (uncommitted at snapshot; resolution path through
`deref_char_proc_ref`, plus the one test that drives a valid stream through
`extract_content_stream_bytes`):**

| Line (worktree) | Test | Pins |
|---|---|---|
| 246 | `resolved_context()` helper | In-memory document (objects 10–22) with hand-built xref offsets; resolver+source `Box::leak`'d to get `'static` |
| 286 | `valid_char_proc_stream_resolves_through_the_parsing_flow` | Object 10 survives resolve+validate, still `ObjectType::Stream` |
| 300 | `valid_char_proc_dict_resolves_through_the_parsing_flow` | **A `/Type`+`/Subtype` dict is a valid char_proc target** — the second half of the §4 decision |
| **310** | **`validated_stream_is_parseable_afterwards`** | **The only test that drives a valid stream through `extract_content_stream_bytes` (line 316).** Validates, then extracts, then asserts the body starts with `b"0 0 10 10 re"` — i.e. validation must not consume/mutate the object and the bytes must survive. Any entry-validation change that makes a *complete* stream fail breaks this. |
| 327–381 | `{integer,string,array,null,boolean,name}_char_proc_is_rejected_with_its_reference` | Objects 12–17 → `InvalidCharProcType { got: "<type> (for ref N 0 R)", expected: "stream or dictionary" }` |
| 387 / 405 / 421 / 437 / 453 | `stream_missing_{type,height,subtype,width}_is_rejected_at_the_resolution_point`, `dict_missing_subtype_…` | Per-key rejection at the resolve seam, each asserting the exact enriched `MissingRequiredKey` key string (e.g. `"/Height (for ref 19 0 R)"`) and the validator's declaration-order (`/Type` before `/Width`) |
| 469 | `a_parser_shaped_dict_is_accepted_even_without_resolution` | Pins the slash-free key convention directly |
| 485 | `unresolvable_reference_is_a_resolution_failure_not_a_validation_one` | Object 999 → `MissingCharProcRef { "999 0 R" }`, distinct from the validation paths |

### 3c. Not this function — do not let a grep fool you

`tests/test_extract_content_stream_bytes.rs` at the **repo root** (6 tests,
lines 47–142) exercises the *content_stream* twin (§5.1) and is **not compiled
by any cargo target**: root `Cargo.toml` is a virtual workspace (`[workspace]`,
no `[package]`), so the root `tests/` dir is not a package test target, and no
`[[test]]` section anywhere points at it. It is grep noise for this bead.

---

## 4. The Dict-arm question — decided, so the next bead does not have to be

`validate_char_proc_structure` (line 388) has an `ObjectType::Dict` branch
(449–476) that **accepts** a dictionary carrying `/Type` and `/Subtype` as a
valid char_proc — pinned by `valid_dict_is_accepted` (integration line 83),
`valid_char_proc_dict_resolves_through_the_parsing_flow` (line 300), and the
in-file `detect_object_type` classification (bf-oufxf7).

A dict has no bytes. So after entry validation lands, a validated-and-accepted
dict flows straight into the `Dict(_)` match arm, and that arm must **still
error** — it may not be "normalized away" as dead code like the scalar arms.

**The error it should become** (pdftract-0914d4cd):

```rust
Type3Error::InvalidCharProcType { got: "dictionary".to_string(),
                                  expected: "stream".to_string() }
```

`"dictionary"` is exactly `PdfObject::type_name()` for `Dict` (types.rs:258), so
the arm can use `type_name()` verbatim. `expected: "stream"` — *not*
`"stream or dictionary"`: by this point the validator has already accepted the
object as a structurally valid char_proc, so the remaining complaint is "a
char_proc that reaches byte extraction must be a stream", and the narrower
string is what distinguishes "valid shape, wrong kind" from the validator's
`Other` arm, which keeps `"stream or dictionary"`.

### The `Indirect` wrinkle (record it now, decide it in the arm-rework bead)

The validator **unwraps** `Indirect` before classifying (lines 389–394), so
`Indirect(valid_stream)` validates `Ok` (pinned by `indirect_stream_is_accepted`).
`extract_content_stream_bytes` takes the object **by value** and does **not**
unwrap: that same `Indirect(valid_stream)` falls into the `Indirect(_)` arm and
returns `Io("Cannot extract stream from indirect object …")` today.

After entry validation this becomes a real inconsistency rather than a latent
one: a value the validator accepts is then rejected two lines later with a
generic `Io`. Two consistent options, either acceptable — pick one and pin it:

1. Unwrap `Indirect` on entry (mirror the validator, e.g. via
   `detect_object_type`-style classification), so an accepted wrapper extracts
   its inner stream; or
2. Keep rejecting it, but as
   `InvalidCharProcType { got: "indirect", expected: "stream" }` — same shape as
   the Dict decision, never `Io`.

In practice `deref_char_proc_ref` cannot hand the function an `Indirect`
(the resolver returns the inner object), so this only bites a caller that
passes a hand-built object — which is exactly what the integration fixtures do.

---

## 5. Gotchas — the ones that likely caused the three earlier failures

### 5.1 There are two `extract_content_stream_bytes`

`crates/pdftract-core/src/content_stream.rs:1817` is a different function with
the same name:

```rust
pub fn extract_content_stream_bytes(
    obj: &PdfObject,
    source: Option<&dyn PdfSource>,
    opts: Option<&ExtractionOptions>,
    doc_decompress_counter: Option<&mut u64>,
) -> Result<Vec<u8>, Diagnostic>
```

It is **not the target**. Quick discriminators: it takes `&PdfObject` (not by
value), returns `Result<_, Diagnostic>` (not `Type3Error`), accepts
`String`/`Array` inputs as *bytes sources* rather than rejecting them, and lives
in `content_stream.rs`. It has its own committed unit tests in
`content_stream.rs` plus the uncompiled root-level file (§3c). Notes from older
beads (bf-2jgkd9, bf-69dimd, bf-36ek2x) describe one or the other — check which
before citing them.

### 5.2 The slash convention — `intern("/Type")` can never match parser output

This is the sharpest edge in the whole bead, and it is **live right now**.

Facts, each pinned in-tree:

1. The lexer consumes the solidus: `lex_name` (`parser/lexer/mod.rs`, "consume
   the leading /") and unit test `name_simple` (mod.rs:1804) pin
   `/Foo` → `Token::Name(b"Foo")`. The module doctest (mod.rs:66) pins the same.
2. `parse_dict` (`parser/object/parser.rs:230–233`) interns the token bytes as-is:
   `<< /Type /XObject >>` produces the key `intern("Type")` — **no slash**.
3. `PdfDict = IndexMap<Arc<str>, PdfObject>` (`parser/object/types.rs:106`), so
   `dict.get("/Type")` is an exact string lookup. A map keyed `Type` does not
   contain `"/Type"`.

Therefore the validator's own required-key lookups —
`type3_rasterizer.rs:411, 419, 427, 435, 457, 465`, all of the form
`stream_dict.get("/Type")` — **can never be satisfied by any dictionary a real
PDF produced**. Every parser-produced stream or dict fails
`validate_char_proc_structure` with `MissingRequiredKey { key: "/Type", … }`,
regardless of what keys it actually carries.

Why the tests have not been screaming about it: the **committed** fixtures in
`tests/type3_charproc_structure.rs` build dicts with `intern("/Type")` — the
same wrong shape — so fixture and lookup agree and the tests pass while
describing no document that exists. That is exactly what the fixture doc
comment means by "the validator got away with lookups nothing could satisfy."
(In-file, `test_deref_char_proc_ref_passes_valid_stream` at line 2928 is in the
same vacuous-pass state, and is *not* being touched by the in-flight work.)

The in-flight sibling edit converts the integration fixtures to slash-free keys
and adds the resolution-path half. **That flips the slashed lookups from
"harmless" to "load-bearing":** once fixtures are parser-shaped,
`valid_stream_is_accepted`, `valid_dict_is_accepted`,
`indirect_stream_is_accepted`, `a_parser_shaped_dict_is_accepted_even_without_resolution`,
`valid_char_proc_stream_resolves_through_the_parsing_flow`,
`valid_char_proc_dict_resolves_through_the_parsing_flow`, and
`validated_stream_is_parseable_afterwards` cannot pass against the current
validator, and the per-key resolution tests (objects 19/21/22) report the wrong
missing key (`/Type` instead of `/Height`/`/Subtype`/`/Width`).

**Ordering consequence for the children:** if pdftract-7a42320d inserts
`validate_char_proc_structure` into `extract_content_stream_bytes` while the
lookups are still slashed, it breaks its own headline case —
`validated_stream_is_parseable_afterwards` (line 310) resolves a *real parsed*
stream, so the inserted validation rejects it with `MissingRequiredKey` and the
bytes are never extracted. Entry validation and the key-lookup convention have
to land together, lookup fix first, or that test must be explicitly
xfail-marked in between.

**Caveat on this section:** derived by inspection, not by executing the suite —
the crate did not compile at snapshot time (§7, WARN-1), so no test in this file
was run for this bead. Each link in the chain above is individually pinned by
in-tree tests; the conclusion follows mechanically. First action for
pdftract-7a42320d, before touching anything:
`timeout --kill-after=30s 600s cargo test -p pdftract-core --test type3_charproc_structure`
to capture the real baseline and confirm or correct this list.

### 5.3 `validate_char_proc_structure` is resolver-less and reports `Ref` as `"unknown"`

The validator takes no `DocumentContext`, so it cannot follow a reference. A
bare `PdfObject::Ref` is reported as `got: "unknown"` — not `"reference"` —
with `expected: "stream or dictionary"` (lines 476–490, bf-5on6og; pinned by
`unresolved_reference_is_reported_as_unknown`, integration line 128). So
validation alone can never clear a `Ref`; only the combination of entry
validation plus the `Ref` arm's recursion covers the indirect case, because the
recursive call re-enters the function and validates the resolved target. Do not
"fix" `"unknown"` to `"reference"` here without revisiting bf-5on6og's
rationale: unknown-not-wrong-type is the deliberate reading of "this check
cannot know."

### 5.4 The function has no production callers

Repo-wide, `extract_content_stream_bytes` (the type3 one) is referenced only by:
its own recursive call at 2255, the unit test at 2975/2984, and
`tests/type3_charproc_structure.rs:316`. Nothing in `pdftract-cli` or any
sibling crate calls it. The production Type3 path gets its bytes a different
way: `font/resolver.rs` ~697 defines `resolve_stream_bytes`, which resolves,
matches `PdfObject::Stream` itself, and calls `decode_stream` directly — it
never touches this function *or* `validate_char_proc_structure`, and passes
`None` for both when it has no context (resolver.rs:741).

Two readings, both useful: (a) low risk — no production behavior changes if the
contract moves; (b) the actual gap — real Type3 charprocs are still unvalidated
at the point bytes are produced, so the umbrella's validation only matters once
this function (or the validator) is wired into the `resolve_stream_bytes` path.
Record which one the umbrella intends before calling the work done.

### 5.5 `Display` prefixes matter when asserting on messages

`Type3Error::Io(msg)` renders as `"I/O error during glyph resolution: {msg}"`
(line 520). Tests that assert `msg.contains("XrefResolver not provided")` are
matching the *inner* string. Any test written against `format!("{}", err)`
instead must include the prefix, and the pinned test at 2975 matches on the
`Err(Type3Error::Io(msg))` variant, so re-typing the error to
`InvalidCharProcType` changes the assertion, not just the text.

---

## 6. Acceptance criteria

| # | Criterion | Status |
|---|---|---|
| 1 | `notes/pdftract-908fe7ff.md` exists, committed, covers points 1–5 | **PASS** — this file; §1 per-arm contract, §2 precondition order, §3 pinned tests, §4 Dict decision, §5 gotchas |
| 2 | Diff touches no `.rs` files | **PASS** — this bead adds `notes/pdftract-908fe7ff.md` only |
| 3 | `cargo check -p pdftract-core` passes | **WARN** — fails, from pre-existing in-flight work this bead did not touch (below) |

**WARN detail (criterion 3):** `cargo check -p pdftract-core` fails with exactly
one error:

```
error[E0061]: this function takes 5 arguments but 4 arguments were supplied
  --> crates/pdftract-core/src/extract.rs:2442
```

`process_content_stream_to_glyphs` gained a 5th parameter
`default_off_ocgs: Option<&HashSet<ObjRef>>` at `extract.rs:190` in an
**uncommitted** sibling edit; its caller at 2442 was not updated in that edit.
`git status` lists `extract.rs` as modified from before this bead started, the
hunk belongs to that diff, and this bead's scope forbids `.rs` changes — so the
fix belongs to whoever owns that edit. Corroborating evidence that nothing else
is wrong: rustc reports **one** error for the whole lib, i.e. every other module
including `font/type3_rasterizer.rs` type-checks, and a markdown note cannot
affect compilation. Consequence: no test could be executed in this iteration
(see the caveat in §5.2); the inventory is from source reading.

---

## 7. Reproduce / re-verify

```bash
# the function, its arms, and the precondition checks
sed -n '2199,2315p' crates/pdftract-core/src/font/type3_rasterizer.rs

# the validator (Dict branch at ~449, Indirect unwrap at ~389, slashed lookups at ~411+)
sed -n '380,490p' crates/pdftract-core/src/font/type3_rasterizer.rs

# lexer solidus behavior and parser key interning
sed -n '1804,1808p' crates/pdftract-core/src/parser/lexer/mod.rs
sed -n '228,234p'  crates/pdftract-core/src/parser/object/parser.rs
grep -n 'pub type PdfDict' crates/pdftract-core/src/parser/object/types.rs

# who calls it (expect: only the recursion at 2255 and tests)
grep -rn 'extract_content_stream_bytes' --include='*.rs' crates/ | grep -v content_stream.rs

# baseline before changing anything (once the tree compiles)
timeout --kill-after=30s 600s cargo test -p pdftract-core --lib font::type3_rasterizer
timeout --kill-after=30s 600s cargo test -p pdftract-core --test type3_charproc_structure
```

Line numbers are from the `0b3e6f16` worktree of 2026-09-07; re-grep if the
sibling edits on this umbrella have landed since.
