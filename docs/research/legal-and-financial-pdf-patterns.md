# Legal and Financial Document PDF Extraction Patterns

PDF text extraction in legal and financial contexts is categorically harder than general document extraction. The document types produced by law firms, courts, accounting firms, and financial institutions share a set of structural conventions that interact poorly with naive bounding-box or stream-order extraction. This document catalogs the patterns pdftract must handle to produce readable, semantically coherent text from these sources.

## Legal Document Structure

Legal documents impose spatial zones that carry semantic meaning independent of their visual appearance. A complaint or contract typically opens with a **caption block** — a formatted header containing party names, court or jurisdiction, and case identifiers — set apart from the body by borders or whitespace. The caption is not prose; it is a structured field cluster. pdftract must recognize caption geometry (centered or left-aligned multi-line blocks in the top third of the first page) and flag the region so downstream consumers can treat it as metadata rather than flowing text.

Numbered paragraphs are the backbone of most legal instruments. Body paragraphs carry explicit numbering (¶ 1, ¶ 2, or bare integers) that defines reading order independently of x/y position. When columns or marginal annotations are present, the paragraph number anchors reconstruction of logical order. pdftract must preserve these numbers in the extracted stream rather than stripping them as decoration.

**Defined terms** appear in two conventions: ALL CAPS (e.g., `AGREEMENT`, `EFFECTIVE DATE`) and bold-faced title case (e.g., **Indemnified Party**). Both signal that the term has a formal definition elsewhere in the document. Extraction must preserve the casing and emphasis signals — stripping ALL CAPS to mixed case or discarding bold metadata silently destroys the term's identity.

Exhibit references (`See Exhibit A`, `attached hereto as Schedule 3.2(b)`) appear inline in body text and at the tail of numbered paragraphs. They are forward pointers into attached or appended documents. pdftract should surface these references with their surrounding context intact so the extraction output carries the logical link.

Signature blocks appear at the end of agreements and at the end of each amendment or addendum. Their spatial form — a grid of underscored lines paired with labels (`By:`, `Name:`, `Title:`, `Date:`) — is distinct from body text and must be flagged as a signature region rather than normalized as prose (see the dedicated section below).

## Court Filing PDFs

US federal and state court filings introduce a margin convention that directly attacks stream-order extraction. California and many other jurisdictions require numbered lines running from 1 to 28 down the left margin of every page. These line numbers are typeset as a separate text column with x-coordinates left of the body text column. A naive extractor reading by x/y order will interleave margin numbers with body words: `1 PLAINTIFF`, `2 respectfully`, `3 submits` becomes `1 PLAINTIFF 2 respectfully 3 submits` — syntactically broken.

pdftract must detect the line-number column by identifying a narrow strip of monotonically increasing integers (1–28) occupying the leftmost 0.5–0.75 inches of each page and exclude it from the primary reading stream. The column can be extracted separately as line-number metadata for applications that need it (e.g., citation tools that reference "line 14 of page 3"), but it must not pollute the prose extraction.

Page headers and footers in court filings carry case numbers, party names abbreviated to fit a single line, and docket identifiers. These repeat on every page and should be extracted once (from the first occurrence) and flagged as repeating header/footer metadata, not as flowing body text. Deduplication across pages is essential; legal briefs can run 50–200 pages with identical headers on every page, and a naive extraction will produce 50 copies of the case caption interleaved into the text.

## Contract Clause Numbering and Hierarchy

Modern commercial contracts use hierarchical numbering schemes that encode the document's logical structure: `1.`, `1.1`, `1.1.1`, then `(a)`, `(b)`, `(a)(i)`, `(a)(ii)`. Some instruments mix Arabic and Roman numerals, parenthetical letters, and unnumbered indented sub-clauses. The extraction challenge is twofold: preserving the numbering tokens themselves, and inferring the nesting depth from indentation so that a consumer can reconstruct the hierarchy.

pdftract must capture the indentation level of each clause by computing the left-margin offset relative to the document's base margin. An increase in left offset combined with a change in numbering style signals a deeper nesting level. The extracted text for a clause like `(a)(i)` should carry metadata indicating it is two levels below its parent section `1.1`, even if the raw character stream contains only the token `(a)(i)` followed by prose.

Clause cross-references (`as defined in Section 8.2(c)`, `subject to Section 4.1.3(b)(ii)`) are high-value in legal extraction. pdftract should preserve these tokens intact — no normalization or abbreviation — because they are the connective tissue of the document's logic.

## Redline and Tracked-Changes PDFs

Redline documents represent negotiation state. They show both the prior text (struck through, typically in red) and the proposed replacement text (inserted, typically in a contrasting color or underlined). When a redline PDF is generated from a word processor, the two versions coexist spatially on the same page.

pdftract must handle redline extraction in at least two modes. In **clean extraction** mode, only the inserted (accepted) text is emitted, and struck-through runs are discarded. In **both-versions** mode, the output interleaves deletion markers and insertion markers so the full negotiation delta is preserved: `[-old text-]{+new text+}` or an equivalent structured representation. Detecting which runs are struck-through requires inspecting text rendering flags or, in tagged PDFs, structure element attributes. For untagged redlines (the majority in practice), horizontal strikethrough lines overlapping text runs are the signal — pdftract must correlate line annotation objects with the text they cross rather than treating them as independent graphical decoration.

Color is a supporting signal but not a reliable primary detector. Firms use different color conventions; some redlines show deletions in red and insertions in blue, others use magenta for one party's changes and green for another's in multi-party negotiations. pdftract should surface color-tagged text runs with their RGB values so downstream logic can apply firm-specific or document-specific color mapping.

## Financial Statement PDFs

Annual reports, audited financial statements, and interim filings are table-dominated. A balance sheet may span three columns (current year, prior year, notes reference) with a header row that spans all three. Income statements carry subtotals, blank separator rows, and grand totals that repeat across column groups.

pdftract's table extraction for financial statements must handle **spanning headers**: a single cell whose text covers two or more columns below it. The physical PDF representation typically places the header text once, horizontally centered over the columns it spans, with no explicit cell boundary in the character stream. Reconstructing the spanning relationship requires measuring the header text's bounding box against the column grid inferred from the data rows below.

Negative numbers in financial statements appear in parentheses: `(1,234,567)`. This convention is distinct from prose parenthetical remarks and must be preserved exactly — converting to a minus sign or stripping the parentheses changes the semantic value. Currency symbols (`$`, `€`, `£`, `¥`) may appear in the first row of a column only, with subsequent rows implying the currency. pdftract should not drop currency symbols or normalize them to a generic marker.

## SEC Filing Patterns

EDGAR filings (10-K, 10-Q, 8-K, S-1, and registration statements) are submitted as HTML or iXBRL and then rendered to PDF by EDGAR's viewer or by the filer. This pipeline introduces conversion artifacts: fonts embedded as image tiles rather than text glyphs, table cells that overflow their bounding boxes and overlap adjacent cells, and hyperlinks rendered as visible URL text that breaks line flow.

Inline XBRL tags (`ix:nonFraction`, `ix:nonNumeric`) do not appear visually in the PDF but may survive as invisible text runs in the PDF character stream if the conversion preserved them. pdftract must strip these zero-width or hidden-layer text fragments from the extraction output rather than treating them as content.

Table of contents pages in SEC filings use dot leaders — rows of periods connecting a section title to a page number. The dots are typeset as a repeating character sequence, not as a tab or graphic rule. pdftract must recognize the dot-leader pattern (a run of `.` or `·` characters spanning most of the line width, followed by a page number) and collapse the run to a single tab-equivalent rather than emitting hundreds of period characters into the text stream.

## Prospectus and Offering Documents

Prospectuses (S-1, S-11, prospectus supplements) use multi-level nested tables to present use-of-proceeds summaries, capitalization tables, and summary financial data. Tables may be nested three levels deep, with outer tables controlling layout and inner tables holding data. Extraction must detect the logical data table within the layout scaffolding and not flatten all cells into a single indistinguishable stream.

Tombstone blocks — the formatted announcement of a securities offering showing issuer, amount, bookrunners, and offering date in a bordered box — appear on cover pages and in marketing materials. Their spatial isolation and internal structure (stacked centered text, often in varying font sizes) distinguish them from body prose. pdftract should flag tombstone geometry as a cover block rather than attempting to integrate it into reading-order prose.

Footnote networks in prospectuses are dense. A single table may carry a dozen footnote markers, with footnotes running across multiple pages. pdftract must associate each footnote marker in the body with its corresponding footnote text, preserving the numeric or alphabetic marker for cross-reference, and must handle footnotes that continue across a page break.

## Invoice and Purchase Order PDFs

Invoices and POs are semi-structured forms with fields occupying fixed regions. Key fields include vendor name and address, customer name and address, invoice number, invoice date, due date, line items (description, quantity, unit price, extended amount), subtotal, tax amount, shipping, and total due. These fields may be laid out in two- or three-column grids with labels left-aligned and values right-aligned or in labeled boxes.

pdftract must extract these as key-value pairs rather than flowing prose. The extraction challenge is that field labels and values are spatially adjacent but may not share a text run — they occupy separate bounding boxes, often with no character-stream relationship. Associating `Invoice No.:` with `INV-2024-00891` requires spatial proximity analysis, not just stream-order reading.

Line item tables in invoices follow a standard grid: each row is one billable item. pdftract's table detection must handle right-aligned numeric columns (where the decimal points or right edges of numbers align, not the left edges of cells) and compute correct column association even when column borders are absent.

## Check and Payment Voucher PDFs

Check images embedded in PDFs or check-layout PDFs present two parallel representations of the payment amount: the numeric amount (`$1,234.56`) and the legal amount in words (`One Thousand Two Hundred Thirty-Four and 56/100 Dollars`). Both must be extracted and surfaced together — they serve different verification purposes and must not be conflated.

The MICR line at the bottom of a check encodes routing number, account number, and check number in MICR E-13B or CMC-7 font. When rendered in a PDF from a scan or a check-printing application, MICR characters may be typeset in a non-standard font that maps to unusual Unicode code points or that requires font-specific glyph remapping. pdftract must handle MICR font substitution and normalize the output to the corresponding digit characters and MICR delimiter symbols (`⑆` for routing, `⑈` for amount).

## Signature Block Detection

Signature blocks follow a spatial template that is nearly universal across legal document types. They appear at the end of the document (or at the end of each signatory section) and consist of one or more parallel columns, each containing an underscored blank line for the actual signature followed by labeled fields: `By:`, `Name:`, `Title:`, `Date:`, and sometimes `Address:` or `Email:`. The underscored line is typically rendered as a sequence of underscore characters or as a drawn horizontal rule.

pdftract must flag signature block regions so that downstream consumers can distinguish them from content. An unfilled signature block should not be extracted as body text at all — the blank lines carry no information. A filled signature block (where names and dates have been typed or handwritten and scanned) presents the fields as labeled key-value pairs and should be extracted as structured metadata: `signatory_name`, `signatory_title`, `signature_date`.

Detection heuristics: a cluster of labels matching the canonical set (`By:`, `Name:`, `Title:`, `Date:`) within a spatial proximity of roughly 1–2 inches, preceded by a horizontal rule or a run of underscores, is a signature block with high confidence. Multiple such clusters arranged side by side indicate multiple signatories. pdftract should emit a signature block record for each cluster rather than treating the region as unstructured text.

---

Together, these patterns define the minimum surface area pdftract must cover to be useful in legal and financial workflows. None of the required behaviors are edge cases — they appear in the majority of documents produced by practitioners in these fields. Correct handling of margin line numbers, clause hierarchy, redline deltas, MICR fonts, dot leaders, and signature regions separates a general PDF extractor from a tool that legal and financial teams can trust.
