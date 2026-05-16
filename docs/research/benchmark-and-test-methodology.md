# Benchmark and Test Methodology for PDF Text Extraction

## 1. Why Benchmarking Matters

PDF text extraction has no agreed-upon standard benchmark. Without one, it is impossible to compare extraction strategies objectively, communicate quality guarantees to users, or detect when a code change causes a regression. A library can claim "high accuracy" while measuring only on clean born-digital PDFs and silently failing on scanned documents or complex table layouts.

A complete benchmark must cover multiple orthogonal quality dimensions:

- **Character accuracy** — are the correct Unicode codepoints recovered?
- **Word accuracy** — are word boundaries preserved after ligature expansion and whitespace reconstruction?
- **Reading order correctness** — does the extracted sequence match human reading order, not PDF paint order?
- **Table structure accuracy** — are row/column relationships preserved across merged cells?
- **Form field extraction** — are AcroForm field names, values, and types correctly recovered?
- **Metadata correctness** — does the XMP/DocInfo metadata round-trip without truncation or encoding errors?

The risk of single-metric optimization is real. Tuning for character error rate on clean PDFs often involves aggressive Unicode normalization that destroys mathematical symbols or CJK ideographs. Tuning for table extraction can introduce extraneous whitespace that degrades WER on prose documents. A benchmark suite must surface these trade-offs rather than hide them.

---

## 2. Ground Truth Corpus Construction

Ground truth can be obtained through four approaches, each with distinct tradeoffs.

**Synthetic PDFs from known text.** A PDF library (e.g., `printpdf`, `lopdf`, or Python's `reportlab`) generates PDFs programmatically from a UTF-8 source string. Because the source is known exactly, comparison is unambiguous and deterministic. Synthetic documents are cheap to generate at scale and cover arbitrary scripts and layouts. Their weakness is that they do not capture real-world PDF quirks: embedded CMaps with broken ToUnicode entries, overlapping glyphs, scanned images masquerading as text layers.

**Manually verified human-labeled PDFs.** A human reads the PDF and produces a ground-truth text file, recording the expected extraction character-for-character. This captures real documents but is expensive: expert annotators typically label 2–5 pages per hour for dense academic material. Inter-annotator agreement for ambiguous whitespace or hyphenation decisions is rarely above 95%, introducing irreducible noise into the ground truth.

**Round-trip from source documents.** When the authoring source is available (LaTeX `.tex` files, Word `.docx`, LibreOffice `.odt`), the plain-text content can be derived from the source rather than re-annotated. LaTeX is particularly clean: stripping macros and math yields the expected prose. The limitation is that PDF layout engines can reflow, hyphenate, and kern text differently from the source, so extracted text is legitimately different from source text without being wrong.

**Crowd-sourced annotation.** Platforms like Amazon Mechanical Turk or Label Studio can produce annotations at scale with majority-vote aggregation. Quality is lower than expert annotation but suitable for coarse WER measurement on large corpora. Reject outlier annotators with high per-document disagreement.

**Minimum corpus size.** For CER/WER to have 95% confidence intervals narrower than ±1 percentage point, a corpus of 500–1000 pages across diverse categories is the practical minimum. Fewer pages produce wide intervals that make small improvements statistically indistinguishable from noise.

---

## 3. Corpus Categories

Different document types stress different extraction code paths. A representative corpus must include:

- **Academic papers** — multi-column layouts, inline math, reference lists with dense hyperlinking, footnotes interleaved with body text.
- **Financial filings** — SEC 10-K/10-Q documents with nested tables, numerical columns, boilerplate legal paragraphs, and XBRL-tagged inline content.
- **Legal documents** — dense prose, numbered exhibits as appendix PDFs, footnotes with hierarchical numbering, redacted (blacked-out) regions.
- **Scanned historical documents** — OCR-rendered image-only PDFs, degraded scan quality, skewed pages, handwritten marginalia.
- **Forms** — AcroForm with checkboxes, radio buttons, combo boxes, text fields, digital signature widgets.
- **Technical manuals** — figures with captions, sidebars offset from main text flow, numbered step lists, code blocks rendered as images.
- **Multilingual documents** — Arabic/Hebrew right-to-left text, CJK ideographs with vertical typesetting options, mixed-script documents.
- **Born-digital word processor output** — PDFs exported from Word, LibreOffice, or Google Docs, representing the dominant document type in enterprise use.

---

## 4. Character Error Rate (CER)

CER is the standard metric inherited from OCR research. It is defined as:

```
CER = (S + D + I) / N
```

where S is substitutions, D is deletions, I is insertions at the character level, and N is the number of characters in the ground-truth string. This is the normalized Levenshtein edit distance between the extracted and reference character sequences.

Before computing CER, normalize whitespace: collapse runs of spaces and newlines into a single space, strip leading/trailing whitespace per paragraph, and optionally Unicode-normalize both strings to NFC. Failing to normalize causes inflated CER from formatting differences rather than extraction errors.

For efficient computation over long documents, use the `rapidfuzz` algorithm (available in the Python `rapidfuzz` crate via FFI, or implement the Wagner-Fischer DP with O(min(m,n)) space). For a 10,000-character document page, naive O(mn) DP is fast enough; for full-document comparisons exceeding 100,000 characters, partition by paragraph and sum.

Report CER broken down by corpus category and compute a weighted overall CER where each category is weighted by its share of the corpus page count. A single overall CER hides category-specific failures.

---

## 5. Word Error Rate (WER)

WER tokenizes both extracted and reference text into word tokens and computes the edit distance at the word level:

```
WER = (S_w + D_w + I_w) / N_w
```

WER is more meaningful than CER for downstream NLP pipelines (named entity recognition, summarization, retrieval) because word-level errors map directly to missed or corrupted tokens.

Tokenization decisions matter. Punctuation attached to words (`"end."`) should be stripped or split into a separate token before comparison — otherwise a missing period inflates WER by creating a substitution (`end.` → `end`). A consistent tokenization scheme must be documented and applied identically to both extracted and ground-truth text.

For CJK scripts (Chinese, Japanese, Korean), word boundaries are not marked by whitespace. WER is undefined without a word segmenter (e.g., MeCab for Japanese, jieba for Chinese). Use CER only for CJK content. For Arabic and Hebrew, apply a morphological tokenizer if available; otherwise use whitespace tokenization with appropriate caveats noted in the report.

---

## 6. Reading Order Accuracy

Extracting correct text is necessary but insufficient if that text appears in the wrong sequence. A PDF stores content streams in paint order, which frequently diverges from reading order in multi-column layouts, sidebars, or documents with footnotes.

The ground truth encodes an explicit word ordering: a sequence `w_1, w_2, ..., w_n` in human reading order. The extractor produces its own sequence `e_1, e_2, ..., e_m`. To measure alignment, compute **Kendall's τ** rank correlation between the ground-truth position of each word and its position in the extracted sequence. τ = 1.0 indicates perfect order; τ = 0 indicates random order; τ = −1.0 indicates fully reversed order.

For documents where word identity is ambiguous (repeated words), use a longest-common-subsequence alignment to match ground-truth words to extracted words before computing rank correlation.

Report per-page reading order τ, and flag pages with τ < 0.8 as layout failures. Two-column academic papers are the canonical hard case and should constitute at least 20% of the reading order sub-corpus.

---

## 7. Table Extraction Metrics

Tables require structure metrics beyond string edit distance. The standard is **TEDS (Tree Edit Distance based Similarity)**:

1. Represent each table as a tree: the root is the table node, children are rows, each row's children are cells. Cells carry `rowspan` and `colspan` attributes and a text payload.
2. Compute the normalized tree edit distance between the extracted tree and the ground-truth tree using the Zhang-Shasha algorithm.
3. `TEDS = 1 − (tree_edit_distance / max(|T_gt|, |T_extracted|))` where `|T|` is the node count.

TEDS ranges from 0 to 1, with 1 indicating perfect structural and content match.

Report TEDS alongside two supplementary metrics:

- **Cell-level text accuracy** — for cells matched by structural alignment, compute CER on cell contents. This separates structural errors from text extraction errors within correctly located cells.
- **Header detection precision/recall** — label which rows are headers in the ground truth, and measure how accurately the extractor identifies them. False-positive header detection (promoting body rows) is the most common failure mode.

---

## 8. Regression Testing Infrastructure

The benchmark corpus is too large to run on every commit. The regression suite is a fast-path subset: 50–100 deterministic PDFs (synthetic PDFs covering edge cases plus a curated set of real PDFs with stable known output) with expected JSON stored in the repository.

Each test case produces structured output:

```json
{
  "pages": [...],
  "metadata": {...},
  "tables": [...],
  "form_fields": [...]
}
```

Use the `insta` crate for snapshot testing. On first run, `insta` captures the JSON output as a committed snapshot file. On subsequent runs, any deviation causes the test to fail and `cargo insta review` presents a diff for human approval. This prevents silent regressions while allowing intentional changes to be reviewed and accepted explicitly.

CI integration uses the Argo Workflows system. The workflow step runs `cargo test` and `cargo insta test --unreferenced=error`, failing the build on any unreviewed snapshot change. The full benchmark suite (all corpus categories, all metrics) runs nightly rather than per-commit, with results posted to a persistent store for trend visualization.

---

## 9. Existing Public Test Corpora

Several public datasets provide ready-made ground truth for specific document categories:

- **PDF Association test suite (pdfa.org/test-suite)** — conformance tests for PDF specification compliance; useful for metadata and structure correctness, not extraction quality.
- **PRImA Layout Analysis Dataset** — scanned newspaper and magazine pages with ground-truth layout regions and reading order. Strong for multi-column layout and region segmentation evaluation.
- **FUNSD** — 199 noisy scanned forms with field-level annotations. Small but directly applicable to form extraction evaluation; free for research use.
- **PubLayNet** — 360,000 academic paper pages from PubMed with region-level annotations (text, title, list, figure, table). Token-level text is not included, but layout regions are.
- **DocBank** — 500,000 academic paper pages from arXiv with token-level annotations extracted by aligning LaTeX source to PDF rendering. The best available resource for reading order and fine-grained text annotation.
- **DeepForm** — 1,500 financial disclosure forms (SEC filings) with field-level ground truth. Useful for financial document extraction and form field accuracy, though the extraction targets are specific named fields rather than full-document text.

Each dataset has limitations: PubLayNet lacks text content; DocBank is academic-only; FUNSD is small and noisy; DeepForm covers a narrow financial niche. A production benchmark corpus should draw from all of them and supplement with synthetically generated documents to fill gaps.

---

## 10. Performance Benchmarks

Extraction quality metrics are necessary but not sufficient. A library that achieves 99% CER at 0.1 pages/second is not production-viable. Track throughput and memory alongside accuracy.

**Metrics to track:**

- **Pages/second** — primary throughput metric; measure on a fixed corpus of representative PDFs.
- **MB/second** — file size throughput; useful for comparing against I/O overhead.
- **Peak RSS per document** — critical for large PDF handling; a document should not require more than 10× its file size in memory.
- **Time-to-first-page** — for streaming APIs; measures latency before any output is available.

Use the `criterion` crate for statistically rigorous benchmarking. Criterion runs each benchmark function multiple times, discards warm-up iterations, and computes mean and confidence intervals. Store benchmark results in a JSON history file (committed or artifact-stored) and compare each run against the baseline commit.

Define acceptable regression thresholds: a throughput drop greater than 5% on the representative corpus triggers mandatory investigation before merge. Memory regressions greater than 10% on any document category also block merge. These thresholds should be enforced in CI by a script that reads Criterion's comparison output and exits non-zero on threshold violation.

Benchmark PDFs must be fixed and versioned — using randomly selected documents introduces variance across runs. Commit a set of 10–20 representative PDFs (covering each corpus category) as binary fixtures in the repository, kept small enough (total < 10 MB) that checkout time is not impacted.
