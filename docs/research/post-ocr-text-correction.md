# Post-OCR Text Correction

Even after a careful font decoding and OCR pipeline, extracted text carries residual errors. Some are systematic — a misencoded font maps the same glyph to the wrong Unicode point on every page. Others are stochastic — noise in a scanned image tips the classifier toward a wrong character. Still others arise from the document structure itself: merged ligatures, broken hyphenation, swapped reading order. Correcting these errors requires layered strategies, applied in the right sequence, with full traceability.

---

## 1. Classes of Errors

**Systematic errors** stem from a consistent encoding mismatch or a trained bias in the OCR classifier. Every occurrence of a glyph produces the same wrong character. Example: a font's `/f_i` ligature slot maps to `fi` but the ToUnicode CMap is absent, so all occurrences render as `ﬁ` (U+FB01) or, worse, as two separate characters with a wrong split point. These are highly correctable once the pattern is identified.

**Random errors** are noise-induced. The classifier assigns a plausible but wrong character with some probability per token. `e` → `c`, `h` → `b`, `a` → `o`. Distribution is roughly uniform over visually similar character pairs; no single substitution dominates.

**Context errors** place the right characters in the wrong positions within a word: `teh` for `the`, `recieve` for `receive`. The characters are individually plausible, just misordered.

**Deletion errors** drop a character: `hous` for `house`. Common at word edges where ink density drops near the margin.

**Insertion errors** introduce a spurious character: `hoouse`. Often from double-ink artifacts or speckling.

**Transposition errors** swap adjacent characters. Distinct from context errors in that only two positions are affected.

Each class responds to a different primary corrector. Systematic errors are best caught by confusable substitution tables. Random errors need dictionary + language model ranking. Context and transposition errors benefit from sequence-level Viterbi correction. Deletion and insertion errors are handled by edit-distance candidate generation.

---

## 2. Dictionary-Based Correction

For each extracted token that fails a dictionary lookup, generate correction candidates within edit distance 1 or 2 using the four standard operations: single-character substitution, deletion, insertion, and transposition. At edit distance 1, the candidate space is `O(|alphabet| * |word|)` substitutions plus `O(|word|)` deletions, `O(|alphabet| * |word|)` insertions, and `O(|word|)` transpositions — manageable. At edit distance 2, enumerate by composing two distance-1 steps; prune candidates not in the dictionary.

Rank candidates by a composite score:

```
score(candidate) = w_ed   * edit_distance(token, candidate)
                 - w_freq * log P(candidate)        // corpus frequency
                 - w_vis  * visual_confusion_cost(token, candidate)
```

`visual_confusion_cost` is low when the substitution involves a known confusable pair (see section 3), high otherwise. This means `l` → `1` is cheaper than `l` → `z`.

**When to correct vs. flag:** Apply a correction automatically when the top candidate's score exceeds a confidence threshold and the edit distance is 1. For distance-2 corrections or low-frequency candidates, emit a flagged suggestion instead. Proper nouns, domain terms, and tokens containing digits are better flagged than blindly corrected.

---

## 3. Confusable Character Tables

Visual confusion in OCR and glyph decoding follows well-known patterns. Build a weighted directed graph where an edge `a → b` carries the empirical probability that OCR produces `b` when the true character is `a`.

Key confusable pairs:

| OCR output | True character | Notes |
|------------|---------------|-------|
| `0` | `O` / `o` | Circular glyph; context (numeric vs. alpha) disambiguates |
| `1` | `l` / `I` | Vertical stroke; `I` lacks serifs in sans-serif fonts |
| `5` | `S` | Curved top; common in low-resolution scans |
| `6` | `b` | Mirrored loop; rare but appears in degraded text |
| `8` | `B` | Closed loops; highly context-dependent |
| `rn` | `m` | Two-stroke sequence visually merges |
| `cl` | `d` | Left stroke + ascender confusion |
| `ii` | `u` | Dot placement ambiguity |
| `vv` | `w` | Double-stroke width |
| `ﬁ` ligature | `fi` | CMap absent; also appears as `f1` |

Rather than exhaustive edit-distance enumeration, use the confusable graph to generate targeted candidates first. For each token, walk the graph: for every character (or digraph) in the token, emit a candidate with the confusable substitution applied. These targeted candidates receive a lower visual cost penalty in the ranking formula, producing higher final scores and reducing false positives relative to generic edit-distance candidates.

Digraph confusables (`rn` → `m`) require special handling: the substitution changes the token length. Track this as a deletion-substitution pair rather than a single-character operation.

---

## 4. Context-Aware Correction with N-gram Language Models

Single-token correction ignores context. The word `sail` and `tail` are both valid English; only the surrounding words disambiguate. A bigram or trigram language model provides a prior over word sequences.

**Noisy channel model:** The corrected sequence `W*` maximizes:

```
W* = argmax_W  P(W) * P(T | W)
```

where `P(W)` is the language model prior and `P(T | W)` is the channel model — the probability that the OCR system produced the observed token sequence `T` given the true text `W`. The channel model is estimated from character-level confusable probabilities: for each position, what is the probability that the true character was substituted, deleted, or inserted to yield the observed character?

**Viterbi algorithm for sequence correction:** Model the correction problem as a hidden Markov model. States are candidate words at each position; transitions are bigram probabilities; emissions are channel model probabilities. Viterbi finds the maximum-probability path in `O(N * K^2)` time where `N` is the token count and `K` is the candidate count per position. In practice, prune candidates to the top 5–10 per position before running Viterbi to keep runtime acceptable.

For trigram models, use a beam search over the lattice rather than exact Viterbi; a beam width of 20–50 balances accuracy against throughput.

**Implementation note for Rust:** Use a pre-built binary n-gram model stored as a hash map from `(word_1, word_2)` to log-probability. A good corpus baseline is a count-based model built from Wikipedia or CommonCrawl, smoothed with Kneser-Ney discounting. Serialize the model as a flat binary file and memory-map it at startup; the hash map itself fits in roughly 2–4 GB for a 3-gram model covering 300k vocabulary.

---

## 5. Regex-Based Pattern Correction

Domain-specific OCR garbles follow predictable patterns that regex handles efficiently, before dictionary lookup ever runs.

Key patterns to encode:

- **Month names:** `1anuary` → `January`, `0ctober` → `October`. Match `[0-9][a-z]{4,8}` at a word boundary; check the digit-substituted form against the 12 month names.
- **Year fields:** `20l8`, `l998`. Match `[12][0-9l][0-9l][0-9l]` and apply `l` → `1` within the match.
- **Large numbers:** `l00,000`, `1,23l,456`. Match sequences of digits, commas, and the letter `l` or `O` surrounded by digits; apply digit confusable substitution within the match.
- **Currency amounts:** `$l.5B1llion`. Tokenize currency prefix + numeric + scale suffix; correct each segment independently.
- **Email addresses:** Validate structure `local@domain.tld`; apply confusable correction only within `local` and `domain` segments, preserving `@` and `.`.
- **URLs:** Match `https?://[^\s]+`; within the matched span, map `O` → `0` and `l` → `1` only when surrounded by other digits.

Regex corrections are deterministic and produce no confidence ambiguity. Apply them before any probabilistic corrector to reduce the token space the language model must consider.

---

## 6. Hyphenation Artifact Removal

PDF text extraction frequently encounters end-of-line hyphens that are typographic (indicating a broken word) rather than semantic (indicating a compound). OCR inherits this artifact from the rasterized layout.

Detection pattern: a span ending in `-` is the last span on a line, and the next line begins with a lowercase token. Steps:

1. Strip the trailing `-` from the first fragment and concatenate with the second fragment.
2. Look up the concatenated form in the dictionary.
3. If found, replace both fragments with the single joined token and update the bounding box to span both original boxes.
4. If not found, apply the Liang-Knuth hyphenation algorithm in reverse: verify that the hyphenation point is a valid break point for the concatenated word. Accept the join if it is; retain the hyphen if it is not.

Preserve explicit hyphens in compound words (`self-referential`) by requiring that the candidate concatenation exists in the dictionary as a non-hyphenated form. When the dictionary is ambiguous, emit both forms as alternatives with confidence scores.

---

## 7. Number and Unit Normalization

Numeric regions in tables and financial text are high-value targets for correction. Apply digit-specific confusable correction after detecting numeric context.

**Numeric context detection:** A span is numeric if it matches `[0-9OolIl,. ]+` and is adjacent to currency symbols (`$`, `€`, `£`, `¥`) or unit suffixes (`%`, `kg`, `mm`, `MHz`). Within a detected numeric span, apply the digit-confusable map: `O` → `0`, `l` → `1`, `I` → `1`, `S` → `5`, `B` → `8`.

**Locale-aware separator handling:** In US/UK locale, comma is the thousands separator and period is the decimal point. In many European locales the roles are reversed. Detect locale from document metadata or from the pattern of separators in the numeric span (a period followed by exactly three digits before another separator is a thousands separator, not a decimal). Apply locale-consistent normalization before emitting the corrected token.

**Formatting preservation:** After correction, re-emit the number in its original format (including comma grouping and decimal precision), not as a normalized float. Callers that need numeric values should parse the corrected string; callers that need the original text appearance should receive the corrected but format-preserving string.

---

## 8. Structural Correction Using Layout

Bounding box metadata enables a class of corrections that pure text analysis cannot reach.

**Reading-order transposition:** When the PDF rendering order does not match visual left-to-right order, extracted words on the same line appear in the wrong sequence. Detect this by comparing `x_min` values of consecutive spans on the same baseline. If span `n` has `x_min > x_min` of span `n+1` but they share the same `y` band, the spans are out of order. Swap them and record a `PositionTransposition` correction.

**Duplicate word detection:** Double-rendering artifacts (a glyph painted twice at slightly offset coordinates) produce duplicate tokens in the same position. Detect pairs of spans with identical text and bounding boxes that overlap by more than 80% of their area. Discard the duplicate; record a `DuplicateRemoval` correction.

**Column boundary errors:** In multi-column layouts, OCR occasionally merges the rightmost word of column 1 with the leftmost word of column 2. Detect by checking whether a token's bounding box crosses a detected column boundary. Split at the boundary and re-tokenize each part.

---

## 9. Correction Pipeline Ordering

The order of correction stages determines correctness. A wrong order produces cascading errors.

```
1. Character-level encoding recovery (font CMap / ToUnicode pipeline)
2. Structural layout correction (position transpositions, duplicates)
3. Regex patterns for known formats (dates, currencies, URLs)
4. Digit confusable substitution in numeric regions
5. Confusable graph candidates for systematic glyph errors
6. Dictionary lookup + edit-distance candidates
7. N-gram context scoring (Viterbi or beam search)
8. Hyphenation artifact joining
9. Number and unit normalization (final formatting pass)
```

**Why this order matters:**

- Dictionary lookup before numeric correction causes `l23` to be matched as a candidate for `lez`, `leg`, or similar — the numeric intent is invisible to the word-level corrector.
- Hyphenation joining after dictionary lookup ensures the joined form is verified against the same dictionary already loaded in memory.
- N-gram scoring after confusable expansion gives the language model a rich but targeted candidate set rather than generic edit-distance noise.
- Structural corrections must precede all text-level corrections; swapped spans in the wrong order produce nonsense input to every downstream stage.

---

## 10. Correction Metadata

Every correction must be traceable. Expose correction records on each span:

```rust
pub struct Correction {
    pub original: String,
    pub corrected: String,
    pub correction_type: CorrectionType,
    pub confidence: f32,       // 0.0–1.0
    pub span_index: usize,
}

pub enum CorrectionType {
    ConfusableSubstitution,
    DictionaryReplacement,
    RegexPattern(String),      // pattern name
    HyphenJoin,
    NumberNormalization,
    PositionTransposition,
    DuplicateRemoval,
}
```

Each `Span` in the extraction result carries a `corrections: Vec<Correction>` field. Page metadata includes a `correction_count: usize` and a `low_confidence_count: usize` (corrections with `confidence < 0.6`) for callers that want a quick quality signal without walking every span.

**Caller policy options:** The extraction API should allow callers to configure a `CorrectionPolicy`:

- `AutoAccept` — apply all corrections above a confidence threshold in the output text.
- `FlagOnly` — return original text with corrections annotated but not applied.
- `ReviewThreshold(f32)` — auto-accept high-confidence corrections, flag the rest.

This lets a downstream LLM pipeline receive clean text directly while a human-review pipeline sees the original alongside the suggestions. The correction metadata is sufficient to reconstruct either representation from the other without re-running extraction.
