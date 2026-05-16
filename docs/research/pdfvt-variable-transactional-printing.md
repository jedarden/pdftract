# PDF/VT Variable and Transactional Printing Document Extraction

## Overview

PDF/VT is an ISO standard (ISO 16612-2) designed specifically for variable and transactional printing workflows. It exists in two conformance levels: PDF/VT-1, which is a single self-contained file based on PDF/X-4, and PDF/VT-2, which supports a file set where page content may reference external files via Reference XObjects. The standard targets high-volume personalized output: direct mail campaigns, monthly billing statements, investment account summaries, insurance policy documents, and utility invoices. A single PDF/VT file may contain thousands of recipient records — each spanning one or more pages — all packed into one bytestream. This structure imposes extraction challenges that flat-page models cannot address adequately.

Where a standard PDF represents a single coherent document, a PDF/VT file is better understood as a batch container. Each record within it is a logically independent document addressed to one recipient. The pages of one record are not meaningfully related to the pages of the next. Extracting the file as a flat sequence of pages and concatenating text produces a result that is structurally meaningless for downstream use: addresses interleave with unrelated balances, transaction rows from different accounts merge into a single stream. pdftract must treat PDF/VT as a record-oriented format and surface an extraction model that matches its intended semantics.

## DPart Hierarchy and Record Enumeration

The structural backbone of PDF/VT is the Document Part (DPart) tree. The document catalog contains a `/DPartRoot` entry pointing to the root of this tree. Each node in the tree is a DPart dictionary; leaf nodes represent individual recipient records. Interior nodes can group records (by region, product line, or processing batch) but the extractable data lives at the leaves.

Each DPart dictionary carries a `/Start` and `/End` entry indicating the first and last page numbers of the pages belonging to that part. To enumerate records, pdftract must walk the DPart tree from the root, recursively following `/DParts` arrays at interior nodes, and collect all leaf nodes in document order. The page range `[Start, End]` for each leaf defines exactly which pages belong to that recipient's document. The DPart tree guarantees that these ranges are non-overlapping and together cover all pages in the file.

The traversal logic cannot assume a fixed tree depth. A billing run may use a two-level tree (root → records), while a more complex campaign may insert grouping levels (root → region → record-batch → records). pdftract's DPart walker must handle arbitrary depth and treat any node with no `/DParts` array as a leaf regardless of depth.

## Document Part Metadata

Each DPart node may carry a `/DPM` (Document Part Metadata) dictionary. At leaf nodes, this dictionary is the primary source of structured per-record data. The `/DPM` dictionary is not arbitrary — it follows an XMP-based schema convention. For transactional documents, it commonly encodes account number, recipient name, mailing address, statement period, amount due, and any segmentation variables used during composition. These fields are present as XMP property paths within an embedded metadata stream.

pdftract should extract the DPM at each leaf DPart and surface it as structured metadata alongside the text content of that record's pages. Because XMP is XML-based, the extraction path is: locate the `/DPM` dict entry in the leaf DPart, retrieve the associated metadata stream, parse the XMP XML, and flatten the relevant namespaces into key-value pairs. The exact namespaces are document-specific — PDF/VT does not mandate a universal schema — so pdftract should emit the raw namespace-prefixed keys and let callers filter for what they need.

This metadata is authoritative for fields like account number and recipient ID. It was written by the composition system before printing and is more reliable than text extracted from the rendered page, which may be subject to font substitution, encoding issues, or layout-driven truncation.

## Variable vs. Static Content

PDF/VT separates variable from static content through two mechanisms: Form XObjects and Reference XObjects. A Form XObject is a self-contained content stream stored once in the file and rendered by reference. A page content stream for one record's page may invoke `/Do` operators to draw the company letterhead, legal footer, or column headers — all stored as Form XObjects that are shared across every record in the file. The variable portion (recipient name, account balance, transaction rows) appears directly in the page content stream or in record-specific Form XObjects referenced only from that record's pages.

For text extraction, this distinction matters because text within a shared Form XObject is static — identical for every recipient — while text in the page's own content stream or in record-local XObjects is the variable payload. pdftract should track XObject usage during extraction and annotate text spans with a `source` field indicating whether the text originates from a shared Form XObject, a record-specific Form XObject, or directly from the page stream. This allows downstream consumers to suppress boilerplate and focus on variable content.

Identifying shared Form XObjects requires tracking which XObjects are referenced from more than one DPart's page set. pdftract can build a reference map during a first pass: for each Form XObject in the file, record the set of pages that invoke it via `/Do`. After DPart enumeration, XObjects invoked from pages belonging to multiple distinct records are static. XObjects invoked exclusively from pages within a single record are record-specific.

## Reference XObjects in PDF/VT-2

PDF/VT-2 allows page content to reference content stored in external files via Reference XObjects. A Reference XObject has `/Subtype /Reference` and carries a `/F` entry pointing to an external file specification and a `/Page` entry indicating which page of that file to use. This enables large static assets (template forms, product images, legal blocks) to live in separate files shared across many PDF/VT-2 print jobs without being duplicated.

pdftract operating in single-file mode — its primary mode for PDF/VT-1 — will not encounter Reference XObjects with external targets. When processing a PDF/VT-2 file, the external files may or may not be present alongside the primary file. pdftract should detect Reference XObjects during content stream parsing. When the referenced file is accessible, pdftract can resolve and inline the referenced content for extraction purposes. When the file is not present, pdftract should record the reference in the output (file specification string, page number) and continue rather than failing. The text contribution of an unresolved Reference XObject is noted as absent with the external reference identifier preserved.

## Postal Address Block Extraction

The first page of each recipient record in a transactional PDF/VT document typically contains a postal address block positioned within a specific bounding region — usually the upper-right quadrant for window-envelope compatibility, or upper-left depending on envelope format. This block contains the recipient's name and mailing address formatted for postal processing.

pdftract should implement position-aware address extraction at the record level. Rather than relying on semantic parsing of free-form text, the extraction should identify the canonical address region by position heuristic: text runs appearing within the upper portion of page one of each record, horizontally offset to the windowed position, and typeset in a distinct font size from surrounding body text. The extracted lines within this bounding box are assembled in top-to-bottom order to form the address block. This region can be configured per document or inferred from DPM metadata if the composition system embeds the address coordinates there.

The address block can be further parsed into structured fields (recipient name, street, city, state, postal code, country) using a lightweight address grammar. For US domestic addresses the USPS-standard structure is reliable; for international addresses, pdftract should emit the raw lines and a `country` hint derived from the last line or from DPM metadata.

## Text That Appears Identical to Static Content

Some PDF/VT composition engines do not use Form XObjects to separate variable from static text. Instead, they generate each page's content stream in full, repeating the static layout text alongside the variable text. In this case, the page content stream for record 47 and the stream for record 48 both contain the full text of the legal footer, column headers, and section titles — copied verbatim — and differ only in the variable fields.

pdftract cannot rely on XObject structure to identify variable content in such files. The DPart tree remains the authoritative guide: text on pages within one DPart leaf belongs to one record, and that is the unit of extraction. For downstream deduplication of static text, pdftract can optionally compute text fingerprints per text run and flag runs that appear identically across more than a configurable threshold of records. These high-frequency runs are likely static template content. This analysis is a post-extraction hint, not a primary extraction feature.

## Extraction Model and Output Schema

The output schema for PDF/VT documents must reflect the record-oriented nature of the format. When pdftract detects a `/DPartRoot` in the document catalog, it switches to record extraction mode. The top-level output is a JSON object with a `records` array. Each element in the array corresponds to one leaf DPart and contains:

- `record_index`: zero-based position in DPart traversal order
- `page_range`: `{ "start": N, "end": M }` using one-based page numbers matching PDF convention
- `dpm_metadata`: key-value pairs extracted from the DPM XMP stream, or null if no DPM is present
- `pages`: array of per-page extraction objects (text spans with position and font metadata, identical in structure to pdftract's standard page output)

The document-level object also carries `dpart_depth`, the maximum depth of the DPart tree, and `record_count`, the total number of leaf DParts. If a `/DPartRoot` is absent, pdftract falls back to flat extraction mode and produces the standard single-document output without a `records` array. This fallback must always be available: not all PDF/VT generators correctly set `/DPartRoot`, and callers processing mixed batches should not require pre-classification of input files.

## Statement and Invoice Documents

The canonical PDF/VT use case — the monthly billing statement — illustrates all of these extraction requirements together. The static frame includes the company name and logo area (text or Form XObject), column headers for the transaction table, legal disclosure text in a reduced font size, and the payment stub layout at the bottom. The variable payload includes the account holder name and address block, account number, statement period dates, each transaction row (date, description, amount), running balance, total due, minimum payment, and payment due date.

For pdftract, the statement extraction goal is to produce per-record JSON objects where the DPM metadata carries the authoritative account number and recipient identity, the address block extraction produces a structured postal address, and the page text spans include the transaction rows tagged with their position data so that tabular structure reconstruction can group them into rows. The transaction table is the highest-value extractable element in a statement PDF — it is the data that downstream reconciliation, audit, and analytics systems need. Correct extraction requires that table rows are associated with the correct record, not bled across a record boundary at a page seam.

pdftract's page boundary handling in record mode must never split a record's pages when assembling text. The page sequence `[Start, End]` from the DPart leaf defines a closed interval; text from page `End` of one record and page `Start` of the next must remain in separate record objects even when those pages are physically adjacent in the PDF page tree.

## Implementation Priority

The foundational requirement is correct DPart tree walking and page-range assignment before any text extraction begins. All subsequent extraction — DPM metadata, address block detection, XObject classification — depends on accurate record segmentation. A PDF/VT file processed without DPart awareness produces output that is technically complete but semantically incorrect for any use case involving per-recipient data. DPart support is not an optional enhancement for pdftract; it is the minimum viable feature for correct PDF/VT handling.
