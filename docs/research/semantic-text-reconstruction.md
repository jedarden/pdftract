# Semantic Text Reconstruction in PDF Extraction

Character-level Unicode recovery and word-level normalization handle the majority of extraction errors, but a class of failures resists both: cases where the raw bytes decode to plausible characters that nonetheless form meaningless or ambiguous text. Fixing these requires semantic context — understanding what the text is *about*, not just how individual glyphs encode. This document describes the algorithms, data structures, and Rust engineering concerns for implementing a semantic reconstruction layer in `pdftract`.

---

## 1. When Semantic Reconstruction Is Needed

Several classes of content systematically evade character- and word-level correction:

**Multi-word technical terms split across encoding boundaries.** A term like "polymerase chain reaction" may straddle a font-switch boundary mid-phrase, producing one half in correct encoding and the other in a shifted code page. Each half looks like a valid English word; only the joined phrase reveals the error.

**Scientific names garbled by font substitution.** Italic species names (*Escherichia coli*, *Drosophila melanogaster*) are often typeset in a separate italic font with its own broken ToUnicode CMap. The substitution produces character-by-character errors that a spell-checker cannot catch because the garbled form may accidentally be a common word.

**Proper nouns and acronyms.** A person's name, an organization acronym, or a product name has no entry in a general dictionary. Extraction errors in these spans go undetected by dictionary lookup. NER provides the discriminating signal.

**Cross-lingual content.** A Latin phrase in an English document (*inter alia*, *habeas corpus*) may be typeset in a decorative font without a ToUnicode entry, producing garbled output. The correct text is not in an English dictionary, but it is in a Latin lexicon and is highly recognizable by n-gram models trained on legal Latin.

**Hyphenated German compounds.** Words like *Verschlüsselungsalgorithmus* or *Haftpflichtversicherung* do not appear in compact dictionaries. Hyphenated splits (*Haftpflicht-versicherung*) add ambiguity: is the hyphen intentional or a line-break artifact? Compound-aware morphological analysis is the only reliable path.

**Mathematical notation in prose.** A formula like "O(n log n)" contains parentheses, letters, and operators whose font encodings frequently diverge. The span must be recognized as mathematical before any correction is applied; applying word-level normalization to a formula destroys it.

---

## 2. N-gram Context Reconstruction

When a span is flagged as low-confidence (below a configurable `confidence_threshold: f32`), the reconstructor enumerates alternative interpretations and scores each using a language model.

**Character n-gram scoring.** A character 5-gram language model, trained on a representative corpus in the target language, assigns a log-probability to each candidate string. For each low-confidence character position, enumerate substitutions constrained by the *character confusable set* — characters whose glyph shapes are known to be confused by common encoding bugs (e.g., `l`↔`1`, `O`↔`0`, `rn`↔`m`, `cl`↔`d`). The confusable set is stored as a `HashMap<char, SmallVec<[char; 4]>>` for O(1) lookup.

**Word n-gram scoring.** After candidate character strings are generated, score them as word sequences using a compressed word bigram or trigram model (ARPA format, loaded as a finite-state acceptor). This promotes candidates that form fluent phrases over candidates that score well character-by-character but form nonsense word sequences.

**Beam search.** Enumerate alternatives using beam search over the character lattice of the low-confidence span. At each position, retain the top-`k` partial hypotheses by cumulative log-probability. Typical values: beam width `k = 16` for character-level search, `k = 8` after word n-gram rescoring. Wider beams improve recall at quadratic cost; the tradeoff is configurable via `ReconstructorConfig { beam_width: usize, max_span_chars: usize }`. For spans longer than `max_span_chars` (default 40), fall back to greedy decoding to bound computation.

**Pruning.** Before scoring, prune hypotheses that violate hard constraints: the candidate must not contain characters outside the Unicode category set observed in surrounding context; the edit distance from the raw decoded form must not exceed a per-character threshold (default 1.5 edits per 10 characters). This eliminates the combinatorial explosion from considering arbitrary substitutions.

---

## 3. Named Entity Recognition for Validation

A lightweight NER model — a CRF or a small transformer quantized to 8-bit integers — classifies spans as `Person`, `Organization`, `Location`, `Date`, `Number`, or `Other`. NER serves two roles: identifying *what kind of thing* a span is, and then applying entity-type-specific validation.

**Type-specific validation.** A span classified as `Date` must parse as a valid calendar date under the document's locale. A span classified as `Number` must be parseable as an integer, decimal, or scientific notation value. A span classified as `Organization` is checked against a pre-loaded organization lexicon. When validation fails, the span is flagged for reconstruction; when it passes, the raw extraction is accepted regardless of character-level confidence.

**Context-conditioned classification.** The entity classifier uses a sliding window of surrounding tokens as context. A span surrounded by financial terminology (ticker symbols, "EBITDA", "basis points") is classified as a financial entity before the span itself is inspected. This reduces false positives where a garbled span accidentally resembles a common word but is semantically a proper noun.

---

## 4. Domain-Specific Lexicons

A general English dictionary misses the vast majority of domain vocabulary. `pdftract` loads supplementary lexicons based on a document classification step that runs before reconstruction.

**Domain lexicon types:** legal (Latin maxims, case citation formats, court names), medical (ICD-10 codes, drug generic and brand names, anatomical terms, lab value abbreviations), financial (ticker symbols, CUSIP/ISIN patterns, ratio names), scientific (IUPAC chemical names, species binomials, journal abbreviations per ISO 4).

**Document classification trigger.** A lightweight bag-of-words classifier (multinomial naive Bayes, ~50 KB model) classifies the document into one or more domains after the first-pass extraction. Domains with posterior probability above 0.6 trigger loading the corresponding lexicon. Multiple domains are possible (a clinical trial report is both medical and statistical).

**Bloom filter storage.** Each domain lexicon is stored as a Bloom filter for O(1) membership queries with bounded false positive rate. A 16-bit cuckoo filter (using the `cuckoofilter` crate or a hand-rolled implementation) achieves a 0.1% false positive rate at 12 bits per entry. Term lookup: normalize the candidate string (lowercase, NFC), query the filter; on a positive hit, optionally verify against a sorted `&[u8]` slice for exact confirmation. Total storage for a 500,000-term medical lexicon is approximately 750 KB.

---

## 5. Cross-Span Consistency

The same visual glyph sequence must extract consistently across the document. Inconsistent extraction — where the same term appears as "photosynthesis" on page 3 and "ph0tosynthesis" on page 17 due to differing font embedding quality — is a common failure mode in multi-section PDFs assembled from separately authored chapters.

**Canonical form selection.** After per-page extraction, group textual spans by their character-level fingerprint: the sequence of Unicode general categories and ASCII characters, with non-ASCII collapsed to a category placeholder. Within each group, select the canonical form as the extraction with the highest summed confidence score; if confidence is tied, use the most frequent string. Write the canonical form to a `HashMap<SpanFingerprint, Arc<str>>`.

**Normalization pass.** In a second pass, any span whose extracted string differs from the canonical form for its fingerprint is replaced with the canonical form and its `reconstruction_method` set to `CrossSpan`. The replacement is only performed when the edit distance between the variant and the canonical form is below a threshold (default: 15% of canonical length), preventing spurious merging of genuinely distinct terms.

---

## 6. Abbreviation Expansion

Abbreviations break sentence boundary detection, inflate vocabulary, and reduce readability. The reconstruction layer handles three kinds:

**Standard abbreviations.** A trie-based lookup (`AhoCorasick` automaton from the `aho-corasick` crate) matches spans against a compiled list of standard abbreviations ("e.g.", "i.e.", "cf.", "op. cit.", "et al."). On a match, the span is tagged with its expansion in the output metadata; the `text` field retains the abbreviated form by default (expansion is opt-in via `ReconstructorConfig { expand_abbreviations: bool }`).

**Document-internal definitions.** The pattern `<full form> (<short form>)` defines a document-local abbreviation. Detected using a regex over the token stream: `\b([A-Z][a-z]+(?: [A-Z][a-z]+)+)\s+\(([A-Z]{2,8})\)`. On detection, insert into a per-document `HashMap<String, String>` mapping short form to long form. All subsequent occurrences of the short form are expanded using this map, with `reconstruction_method` set to `DocumentAbbrev`.

**Ambiguous period handling.** A period following a known abbreviation is not a sentence boundary. This table is shared between the abbreviation expander and the sentence boundary detector.

---

## 7. Reference and Citation Reconstruction

Bibliography sections have distinct extraction failure patterns: author names are frequently reordered or truncated, journal titles run together with volume numbers, and DOIs are broken by line-wrap hyphenation.

**Zone detection.** The bibliography zone is identified by a combination of signals: high density of year-pattern tokens (four-digit sequences in the range 1900–2050), high density of capitalized multi-word sequences, and a section header matching a list of known bibliography heading strings. Once identified, reconstruction rules specific to the reference zone are applied.

**Structural validation.** A DOI must match `10\.\d{4,9}/[-._;()/:A-Za-z0-9]+`. An ISSN must match `\d{4}-\d{3}[\dX]`. A year must parse as an integer in the range 1700–2100. When a span in the reference zone partially matches one of these patterns but contains obvious character substitutions (e.g., `l0.1038/...` where `l0` should be `10`), apply a targeted correction using the pattern as a template.

**DOI normalization.** Hyphens inserted by PDF line-wrapping inside a DOI are detected by splitting on hyphen and checking whether reassembly yields a valid DOI regex match. If so, remove the hyphen.

---

## 8. Sentence Boundary Detection

Periods are the most ambiguous character in prose text. A period may end a sentence, terminate an abbreviation, separate decimal digits, form an ellipsis, or appear inside a URL or DOI. A rule-based sentence boundary detector resolves ambiguity in order:

1. If the preceding token is in the abbreviation table, the period is not a sentence boundary.
2. If the preceding token is a single uppercase letter (initials), the period is not a sentence boundary.
3. If the following token begins with a lowercase letter, the period is not a sentence boundary.
4. If the period is followed by another period (ellipsis), it is not a sentence boundary.
5. Otherwise, the period is a sentence boundary.

**PDF-specific complications.** A line break in a PDF content stream does not imply a sentence break. After geometry-based line joining (handled in an earlier normalization stage), the sentence detector operates on joined paragraphs, not raw lines. Hyphenated line-end tokens that have been rejoined must not present a spurious word boundary to the detector.

---

## 9. Coherence Scoring for Reconstruction Candidates

When multiple reconstructions of a passage are plausible, select the best using a composite score:

- **(a) Word n-gram perplexity.** Lower perplexity is better. Computed using a trigram model with Kneser-Ney smoothing.
- **(b) Part-of-speech sequence probability.** Tag the candidate with a fast POS tagger (e.g., averaged perceptron); score the POS sequence under a bigram POS language model. Promotes syntactically coherent candidates.
- **(c) Entity consistency.** Count entity type conflicts between the candidate and the surrounding 200-token context window. A candidate that introduces an unexpected entity type (e.g., a location in a paragraph that is otherwise about persons) is penalized.
- **(d) Semantic similarity.** Encode the candidate and the surrounding paragraph using a compact sentence embedding model (e.g., a 4-layer distilled transformer, ~25 MB) and compute cosine similarity. Candidates that are semantically distant from their context are penalized.

The composite score is a weighted sum: `score = w_ppl * ppl + w_pos * pos_cost + w_entity * entity_penalty - w_sem * cos_sim`. Weights are configurable; default values are calibrated on a mixed-domain PDF corpus.

---

## 10. Output and Confidence

Each reconstructed span in the `pdftract` output carries the following fields:

```rust
pub struct ReconstructedSpan {
    /// Raw bytes as decoded before reconstruction, percent-encoded if non-UTF-8.
    pub original_raw: String,
    /// Final reconstructed string, normalized to NFC.
    pub text: String,
    /// True if any reconstruction algorithm modified `text` relative to `original_raw`.
    pub reconstruction_applied: bool,
    /// Confidence in the reconstructed text, in [0.0, 1.0].
    pub reconstruction_confidence: f32,
    /// The primary algorithm responsible for the reconstruction.
    pub reconstruction_method: ReconstructionMethod,
}

pub enum ReconstructionMethod {
    None,
    Dictionary,
    Ngram,
    Entity,
    CrossSpan,
    DomainLexicon,
    DocumentAbbrev,
}
```

**Page-level metrics.** Each `ExtractedPage` carries a `reconstruction_rate: f32` — the fraction of spans on that page for which `reconstruction_applied` is true. A page with `reconstruction_rate > 0.3` should be flagged in the caller's output as a low-quality extraction, potentially warranting an OCR fallback. The `ReconstructionMethod` distribution across a page (accessible via `page.reconstruction_method_histogram()`) gives the caller a breakdown of *why* reconstruction was applied, which is useful for diagnosing systematic problems in a PDF batch.

When `reconstruction_applied` is false, `reconstruction_confidence` reflects the confidence of the original extraction, not of the reconstruction pass. This preserves the meaning of the field: it always represents the extractor's confidence in `text`, not in the decision to reconstruct.
