# Chunking for LLM Consumption

**Project:** pdftract — Rust PDF text extraction library  
**Scope:** Algorithms and output formats for chunking structured extraction results into LLM-ready segments

---

## 1. Why Chunking Is a pdftract Concern

A PDF extraction pipeline typically ends at flat text. The consuming application then applies chunking — splitting that text into segments sized for embedding models or LLM context windows. This division of labor has a significant defect: the chunker must re-infer structure that the extractor already computed.

pdftract operates at the block level. Each block carries a `kind` (paragraph, heading, table, figure, footnote, list item), a `bbox` (bounding box on the page), a `zone` label (body, sidebar, header, footer, caption), and full Unicode text. These properties encode the semantic structure of the document. A paragraph boundary in pdftract's output is not a heuristic — it is derived from the PDF's glyph stream geometry and, where present, its logical structure tree.

A downstream chunker working from a flat string has none of this. It must guess paragraph boundaries from double newlines, infer heading levels from font-size differences it cannot see, and split tables it cannot identify. Every inference the downstream tool makes is a degraded approximation of what pdftract already resolved with precision.

The practical consequence is chunk contamination: a heading gets merged with the paragraph preceding it from the previous section; a table row straddles a chunk boundary; a footnote bleeds into body text. Each of these reduces embedding quality and retrieval precision.

Exposing chunking as a built-in output mode — a `--mode chunks` flag or a `chunks` field in the JSON envelope — allows pdftract to apply its structural knowledge directly. The semantic boundaries are already known at extraction time. Chunking is the correct layer at which to use them.

---

## 2. Semantic Boundary Types

pdftract identifies several block transition types that make natural, high-quality chunk boundaries:

- **Heading transitions.** A block with `kind: heading` at any level (H1, H2, H3) marks the start of a new document section. This is the strongest semantic boundary available.
- **Paragraph breaks.** Adjacent paragraph blocks with no heading between them represent continuous prose in the same section. The gap between them is a valid split point.
- **Table boundaries.** A `kind: table` block is a self-contained unit with a defined start and end. Splitting inside a table loses row coherence and column semantics.
- **Figure and caption units.** A `kind: figure` block paired with an adjacent `kind: caption` block should be kept together. Separating them makes the caption uninterpretable in retrieval.
- **Footnote blocks.** `kind: footnote` blocks often belong to specific body paragraphs by reference number. They are candidates for inclusion with their referencing paragraph or for separate indexing, but should not straddle arbitrary boundaries.
- **List boundaries.** A sequence of `kind: list_item` blocks forms a unit. Splitting a list mid-item degrades readability and breaks the syntactic completeness of the item.

Each of these is already labeled in pdftract's block output. A chunker with access to the block stream can use these labels directly without any re-inference.

---

## 3. Fixed-Size Chunking with Semantic Snapping

The baseline chunking strategy targets a maximum of N tokens per chunk. Naive fixed-size chunking splits at exactly N tokens, producing fragments that end mid-sentence or mid-paragraph.

Semantic snapping improves on this: accumulate blocks until the token budget is reached, then extend or retract to the nearest clean semantic boundary before closing the chunk. In practice, this means:

1. Accumulate blocks in order.
2. After adding each block, check whether the running token estimate exceeds the target.
3. When it does, close the chunk at the end of the current block (if the block itself is within budget) or at the last sentence boundary within the current block's text.
4. Begin the next chunk at the start of the next block.

This approach keeps block integrity. A paragraph that fits within the budget is never split. A paragraph that exceeds the budget is split at a sentence boundary — identified by terminal punctuation followed by whitespace — rather than at a character offset.

Blocks larger than the target chunk size (long tables, large prose paragraphs) require special handling. For prose blocks, split on sentence boundaries and emit each sentence group as its own chunk, preserving the block's metadata (page, zone, heading context) on every sub-chunk. For table blocks, see Section 6.

---

## 4. Heading-Based Hierarchical Chunking

Heading-based chunking uses H1/H2/H3 transitions as primary split points, producing chunks that correspond to document sections rather than token windows.

The algorithm builds a document tree from the heading block sequence:

1. Scan the block stream in order.
2. When a heading block is encountered, push it onto a heading stack, popping any heading at the same or lower level (H2 pops a preceding H2 but not a preceding H1).
3. Accumulate subsequent non-heading blocks as children of the current heading node.
4. Each leaf node (heading + its body blocks) becomes a chunk candidate.

Every chunk inherits the full heading path from root to its immediate heading, forming a breadcrumb: `["Introduction", "Background", "Prior Work"]`. This breadcrumb is included in the chunk's metadata and optionally prepended to the chunk text so that embedding models encode the section context alongside the content.

For very large sections (a single H2 section spanning 4,000 tokens), hierarchical chunking falls back to paragraph-boundary splitting within the section, carrying the heading breadcrumb forward on each sub-chunk.

Documents with no headings degrade gracefully to paragraph-boundary chunking. The heading breadcrumb is omitted or replaced with a page-range label.

---

## 5. Sliding Window with Overlap

RAG retrieval systems suffer from boundary loss: a query whose answer spans two adjacent chunks retrieves neither chunk with high confidence because the relevant content is split across a boundary. Sliding window chunking with overlap addresses this by including a suffix of the previous chunk at the start of the current one.

Typical overlap sizing is 10–20% of the target chunk size. For a 512-token target, 64–100 tokens of overlap is standard. Overlap beyond 25% produces diminishing returns while significantly inflating index size.

Semantic snapping interacts with overlap in a non-obvious way: the overlap region should not begin mid-sentence. When computing the overlap suffix, walk backward from the chunk boundary to the nearest sentence start, then include from that point forward. This ensures the overlap text is syntactically complete and embeds correctly.

Overlap helps when:
- Queries target local context (a specific fact, a named entity, a numeric value) that might fall near a chunk boundary.
- Documents are dense prose with high local coherence.

Overlap hurts when:
- Documents are primarily tabular or list-based (overlap duplicates structured data without semantic benefit).
- The embedding model has a very short context window (overlap consumes budget needed for content).
- Index size is a hard constraint (every overlapping token appears in two chunk embeddings).

pdftract's block structure supports overlap implementation cleanly: overlap is measured in blocks (include the last M blocks of the previous chunk at the start of the current one) rather than in raw characters, preserving semantic integrity.

---

## 6. Table Handling in Chunks

Tables require special treatment because row-level coherence is critical for embedding quality. Three strategies are viable, each with distinct tradeoffs:

**A. Whole-table as single chunk.** Emit every table as one chunk regardless of size. This preserves row and column relationships completely. The drawback is unbounded chunk size — a 200-row financial table becomes a single embedding that may exceed model context limits and produces a coarse retrieval unit.

**B. Row-boundary splitting with header repetition.** Split the table into N-row segments, repeating the header row at the start of each segment. This bounds chunk size while preserving column semantics. The repeated header adds token overhead (proportional to column count and row segment count) but makes each sub-chunk independently interpretable. This is the recommended strategy for wide or long tables.

**C. Serialize as markdown within surrounding prose.** Convert the table to GitHub-flavored markdown and include it in the prose chunk that precedes or follows it. This works well for small tables (2–5 rows) embedded in analytical text where the table is subordinate to the prose argument. It fails for large tables where the serialized markdown dominates the chunk and overwhelms the prose context.

The appropriate strategy depends on table size and document type. pdftract can expose a `table_chunk_strategy` parameter with values `single`, `row_split`, and `inline_markdown`.

---

## 7. Token Budget Awareness

Chunk size must be measured in tokens, not characters, because language models have token-count context limits and embedding models have token-count input limits. The character-to-token ratio is not fixed: English prose averages roughly 4 characters per token under byte-pair encoding; CJK text averages 1–2 characters per token due to high-entropy characters that do not merge into multi-character tokens.

pdftract should implement a fast token estimator that does not depend on a specific model's tokenizer. A practical approach:

- For ASCII-dominant text, estimate `token_count ≈ char_count / 4.0`.
- For text with high Unicode density (detected via codepoint range sampling), adjust the denominator toward 1.5–2.0.
- For mixed content, compute a weighted average based on character class proportions.

This estimate is exposed as `token_estimate` in chunk output and used internally to enforce `max_tokens` budget limits. The estimate is intentionally conservative (slightly over-counts) to avoid producing chunks that overflow model context limits at inference time.

`max_tokens` should be a first-class chunking parameter alongside `strategy` and `overlap_tokens`.

---

## 8. Metadata per Chunk

Every chunk emitted by pdftract must carry the following metadata fields:

- **`pages`** — the 1-indexed page range covered by the chunk's source blocks.
- **`heading_breadcrumb`** — ordered array of heading texts from the document root to the section containing this chunk.
- **`zone`** — the dominant zone label of the chunk's blocks (`body`, `sidebar`, `header`, `footer`, `caption`). Determined by the zone label appearing in the majority of the chunk's blocks by character count.
- **`char_offset_start` / `char_offset_end`** — character offsets into the full document text (defined as the concatenation of all block texts in document order). These enable citation generation: given a chunk retrieved by a RAG system, the citing application can locate the exact span in the source document.
- **`chunk_index`** — zero-indexed position of this chunk in the full chunk sequence.
- **`total_chunks`** — total number of chunks emitted for the document.

This metadata feeds retrieval ranking (prefer body-zone chunks over sidebar-zone chunks for general queries), citation generation (reconstruct the page and paragraph reference), and debug inspection (verify chunk boundaries align with document structure).

---

## 9. Late Chunking Compatibility

Late chunking is a retrieval technique where the full document is passed to a long-context embedding model and the resulting token embeddings are pooled per chunk region after the forward pass. This preserves global document context in local chunk embeddings — a quality improvement over independent chunk embedding.

Late chunking requires two things from the extraction layer: (a) the full document text as a single string, and (b) the character or token offsets of each chunk within that string, so that the post-pass pooling step knows which embeddings to aggregate.

pdftract can expose a `full_text_with_offsets` mode that emits:
1. A single `full_text` string — the concatenation of all block texts in reading order with standardized separators.
2. A `chunks` array where each entry contains only `char_offset_start`, `char_offset_end`, and the metadata fields from Section 8 (no repeated text).

The consuming application passes `full_text` to the embedding model and uses the offset array to pool the resulting embedding matrix. This decouples chunking strategy from the embedding call, allowing the same pdftract output to drive both standard independent-chunk embedding and late-chunking pipelines without re-extraction.

---

## 10. Output Format

When chunking mode is enabled, pdftract emits a top-level `chunks` array in its JSON output. Each element conforms to:

```json
{
  "chunk_index": 0,
  "total_chunks": 42,
  "text": "...",
  "token_estimate": 380,
  "pages": [3, 4],
  "heading_breadcrumb": ["Introduction", "Background"],
  "zone": "body",
  "char_offset_start": 1240,
  "char_offset_end": 2890
}
```

Field semantics:

| Field | Type | Description |
|---|---|---|
| `chunk_index` | integer | Zero-based position in the chunk sequence |
| `total_chunks` | integer | Total chunks in this document |
| `text` | string | The chunk's full text content |
| `token_estimate` | integer | Estimated token count (conservative BPE estimate) |
| `pages` | integer[] | 1-indexed page numbers spanned by this chunk |
| `heading_breadcrumb` | string[] | Heading path from document root to this chunk's section |
| `zone` | string | Dominant zone label of source blocks |
| `char_offset_start` | integer | Start offset in the full document text string |
| `char_offset_end` | integer | End offset (exclusive) in the full document text string |

The `text` field is omitted in `full_text_with_offsets` mode, where the consuming application derives text from the full document string using the offset pair.

Chunking parameters are specified in the extraction request:

```json
{
  "strategy": "heading_hierarchical",
  "max_tokens": 512,
  "overlap_tokens": 64,
  "table_chunk_strategy": "row_split"
}
```

Valid `strategy` values: `fixed_size`, `heading_hierarchical`, `sliding_window`, `full_text_with_offsets`.
