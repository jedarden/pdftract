# Article Threads, Reading Order Override, and Multi-Page Flow

## Overview

PDF article threads are one of the oldest and least-implemented features of the format, introduced in PDF 1.1 as a mechanism to guide readers through content that flows across non-contiguous page regions. For tools focused on extracting text in logical reading order, article threads represent both an underused signal and a practical necessity when processing magazine-style, newsletter, or column-heavy documents where page-by-page extraction produces fragmented, incoherent output.

## The /Threads Structure in the Document Catalog

The document catalog — the root object of every PDF — may contain a `/Threads` array. Each entry in this array is a Thread dictionary, representing a single logical article or content flow. The Thread dictionary has two keys: `/F`, which points to the first Bead dictionary in the thread's linked list, and an optional `/I` (info) dictionary that carries metadata such as the article's title and an identifier.

Bead dictionaries are the atomic units of a thread. Each Bead has five relevant keys:

- `/T` — a reference back to the enclosing Thread dictionary (used for traversal integrity checks)
- `/N` — the next Bead in sequence; this is a direct object reference, not an index
- `/V` — the previous Bead (doubly linked for bidirectional traversal)
- `/P` — a reference to the Page object on which this bead's content appears
- `/R` — a rectangle in page default user space, specifying exactly which region of that page contains this bead's content

To traverse a thread, pdftract starts at `/F` and follows the `/N` chain until it circles back to the first bead (the list is circular) or until `/N` is null, depending on the authoring tool. The termination condition must be implemented defensively: track visited bead object numbers to avoid infinite loops in malformed files, and treat a null `/N` or a bead already seen as the end of the sequence.

## Why Threads Matter: Magazine Flow Across Non-Consecutive Columns

In a typical magazine layout, a single article might begin in columns one and two on page three, jump to a sidebar region on page seven, and conclude on page nine's inner column. A page-by-page extractor — even one that correctly orders text by reading direction within a single page — will interleave that article with every other article that appears on pages three, seven, and nine. The result is unusable for downstream processing, summarization, or indexing.

Article threads encode the editorial intent directly in the file. When a publisher's layout application writes threads, it is asserting that the text within bead rectangle R1 on page P1, followed by the text within bead rectangle R2 on page P2, is a single coherent unit. This is information that cannot be recovered from glyph positions alone without a full layout analysis engine. pdftract must extract and honor this signal when it is present.

## Extracting Text from Bead Rectangles

For each bead, the extraction process is as follows. First, resolve the `/P` reference to obtain the Page object. Then, apply the page's coordinate transform — accounting for `/MediaBox`, `/CropBox`, any `/Rotate` value, and the CTM established by the page's content stream — to bring the bead's `/R` rectangle into the same space used during text operator processing. The `/R` value is specified in page default user space before rotation, so the same coordinate normalization logic used for text glyph positions must be applied. A bead rectangle specified as `[x1 y1 x2 y2]` in user space must be tested against the bounding box of each text span on the page after both the text matrix and the current transformation matrix have been applied to the glyph's origin.

For each text span or glyph cluster on the page, test whether the glyph's position falls within the transformed bead rectangle. Collect all qualifying spans, sort them by the normal reading order for that page (top-to-bottom, then left-to-right for LTR scripts), and concatenate them into the bead's text contribution. Repeat for every bead in thread order. The concatenation of bead texts, in `/N`-chain sequence, produces the reconstructed article.

Word boundaries that straddle the bead rectangle edge require care. If a glyph cluster partially overlaps the rectangle, pdftract should include it if the glyph's reference point (typically the lower-left of the advance rectangle) falls within the bounds. Half-pixel tolerance may be appropriate to handle floating-point imprecision in coordinate storage.

## Multiple Threads per Document and Overlapping Regions

A single PDF document may contain many independent threads, each representing a separate article. pdftract must iterate the entire `/Threads` array and process each thread independently. The output structure should maintain strict separation between thread content and page-body content.

A single page region may be covered by beads from more than one thread — a common pattern for pull quotes, sidebars, and boxed callouts that are simultaneously part of the surrounding article flow and their own thematic thread. When a glyph falls within rectangles belonging to multiple threads, pdftract should assign it to all matching threads. The downstream consumer — not the extractor — is better positioned to decide how to handle the overlap, whether to deduplicate or to preserve the multiple associations.

Text on a page that does not fall within any bead rectangle is classified as page body content and is extracted under the normal page extraction pathway. This partitioning allows pdftract to produce both a threads-based view and a page-based view from the same document without discarding content that the thread definitions do not cover.

## SpiderInfo and Thread Metadata

The optional `/I` dictionary attached to a Thread — sometimes called the SpiderInfo dictionary — may carry a `/Title` string and an `/ID` string. These were originally intended to support web spider crawlers during the era when PDF files were indexed by content-aware search agents. Today, they provide a practical mechanism for extracting article metadata: a `/Title` of "Feature: The Architecture of Memory" attached to a thread lets pdftract label the extracted content without heuristic guessing.

When `/I` is present and `/Title` exists, pdftract should use it as the `title` field in the thread output object. When absent, the field should be null rather than a synthesized value. The `/ID` field, when present, is a byte string and may be used as a stable identifier if the document is re-extracted; it can populate the `thread_id` field in the output schema.

## Output Schema for Thread Content

pdftract's JSON output should include a top-level `threads` array, present when the document contains at least one thread. Each entry in the array is an object with the following fields:

- `thread_id` — the `/ID` value from `/I` if present, otherwise the zero-based index of the thread in the `/Threads` array
- `title` — the `/Title` value from `/I` if present, otherwise null
- `bead_text` — an array of strings, one per bead in traversal order, each containing the extracted text for that bead's rectangle

This schema makes the bead structure visible to consumers who need to understand the original flow segmentation, while also allowing simple concatenation of `bead_text` entries for consumers that only want the article as a flat string. Page and line metadata for each bead can be added as parallel arrays without breaking existing consumers.

## Priority: Tagged PDF vs. Article Threads

When a document carries both a tag tree (Structure Tree Root with semantic tags such as `<Article>`, `<P>`, `<Sect>`) and article threads, the structure tree is the authoritative source for reading order. Tagged PDF was designed precisely to encode logical document structure in a machine-readable form, and modern accessibility-compliant exports rely on it. Article threads are a parallel mechanism that predates tagged PDF by several versions and carries less semantic granularity.

pdftract's extraction pipeline should therefore apply the following priority: if a document is tagged, use the structure tree as the primary reading order source and treat article threads as supplementary metadata. If a document is untagged but has article threads, elevate the threads to the primary ordering mechanism for multi-flow content. If a document has neither, fall back to geometric heuristics for column detection and reading order reconstruction.

This fallback chain should be detectable and exposed in pdftract's output as an `extraction_strategy` field at the document level, allowing consumers to understand which mechanism was used and calibrate their confidence accordingly.

## Coordinate Transforms and Precision

The `/R` rectangle in a Bead dictionary is expressed in page default user space. This is the coordinate system that exists before the page's `/Rotate` entry is applied. When a page has a 90-degree or 270-degree rotation — common in landscape-oriented magazine pages — the bead rectangles must be rotated by the same angle before being compared against glyph positions, which are computed in the post-rotation display space. Failing to apply this transform will cause bead regions to miss all the text they should capture.

The page's `/MediaBox` origin must also be subtracted before comparison if it does not start at `[0 0]`. Some authoring tools produce non-zero media box origins, and bead rectangles are specified relative to the media box origin. The same normalization applied to glyph coordinates in the main extraction path must be applied identically to bead rectangle coordinates to ensure consistent intersection testing.

## Graceful Fallback for Legacy and Modern Documents

Article threads are a PDF 1.1 feature with a long history, but modern layout applications such as InDesign have largely abandoned them in favor of tagged PDF for accessibility compliance. A document produced by a current version of InDesign will typically contain a rich structure tree and no `/Threads` array at all. pdftract must handle the absent-threads case without error: if the document catalog does not contain a `/Threads` key, or if the array is empty, the `threads` field in the output is either omitted or set to an empty array, and extraction proceeds through the normal pathways.

For documents that do contain threads — primarily older magazine PDFs, documents from legacy publishing workflows, and some scanned-and-OCR'd publications where threads were added post-hoc by a PDF processing tool — pdftract's thread extraction provides the only reliable way to recover the intended reading order without reimplementing a full layout analysis engine. Detecting presence, traversing the linked list safely, applying correct coordinate transforms, and partitioning page content by bead coverage are the four implementation requirements that make thread-aware extraction work correctly for this class of document.
