# Government Form and Regulatory PDF Extraction Patterns

## Overview

Government-origin PDFs represent one of the most structurally diverse and extraction-challenging categories a PDF library will encounter. Unlike commercial documents produced by a single authoring toolchain, government forms span decades of software, print-and-scan workflows, AcroForm interactivity, security paper, barcodes, and classification markings. pdftract must handle each of these patterns correctly to produce complete, usable text output rather than partial or silently incorrect extractions.

---

## IRS Tax Form PDFs

IRS forms such as the 1040, W-2, and the Schedule series are AcroForm PDFs with named field annotations. Each numbered line — Line 1, Line 7a, Line 22b — corresponds to a distinct AcroForm widget. pdftract must extract both the field name (as a structured label) and the field value, preserving the line numbering as a key into the extracted record. Checkbox fields for filing status (Single, Married Filing Jointly, Head of Household, and so on) carry a Boolean value in the field annotation and must not be confused with adjacent label text.

A critical edge case arises when a taxpayer prints a partially completed e-file form, fills in handwritten amounts, and rescans it. In this case the AcroForm values are absent — the form is now a scanned image — and computed totals that would otherwise appear in widget values are only recoverable through OCR of the scanned pixel layer. pdftract must detect the absence of AcroForm data on a form that structurally resembles a known AcroForm template and escalate to OCR rather than returning empty fields. Line-position heuristics (vertical Y coordinate buckets aligned to IRS layout grids) can recover labeled numeric values even when OCR confidence is imperfect.

Schedule attachments (Schedule B, Schedule D, Schedule SE, and so on) are typically embedded as additional pages within the same PDF file. pdftract should preserve page-level provenance — attaching each extracted field to the page index from which it came — so callers can distinguish Form 1040 page 1 data from Schedule D capital gains tables.

---

## Immigration Form PDFs (USCIS I-Series and N-400)

USCIS forms such as the I-130, I-485, I-765, and N-400 follow a predictable multi-part section structure labeled with capital letters: Part 1, Part 2, Part 3, and so on, each subdivided into numbered items. pdftract should recognize this section hierarchy and expose it in extraction output as a nested structure keyed on part and item number, not merely as a flat ordered list of field values.

Checkbox fields in USCIS forms carry high semantic weight. A checkbox for "Yes" or "No" in response to a question about criminal history, prior immigration violations, or membership in prohibited organizations is legally significant. pdftract must preserve checkbox state — checked or unchecked — and associate it unambiguously with the parent question text. When multiple checkboxes appear within a single question (for example, "check all that apply"), each must be individually annotated.

Signature pages present a distinct challenge. The signature itself is typically an image or a user-drawn annotation; pdftract should flag the signature field as `signature_field` in metadata and extract the surrounding attestation text (the printed legal declaration above the signature line) as normal text. It is never correct to suppress or skip signature pages.

Barcode pages are appended to USCIS forms when generated through the USCIS online filing system or certain immigration software packages. These pages contain a PDF417 or similar 2D barcode encoding the entire form submission as a binary payload. pdftract detects such pages, flags each detected barcode region as `barcode_detected` with its bounding box coordinates, and does not attempt to decode the binary payload as text. Any human-readable data printed adjacent to the barcode — a confirmation number, applicant name, or form identifier — is in the normal vector text layer and is fully extractable.

---

## US Passport and Visa Application Forms

The DS-11 (passport application) and DS-160 (nonimmigrant visa application) follow a biographic data field pattern: surname, given name, date of birth, place of birth, Social Security Number, and travel document details. These fields are either AcroForm widgets or pre-labeled grid cells depending on the version and whether the form was electronically generated or pre-printed.

A photograph placeholder occupies a designated rectangular region on these forms. The placeholder is an image container, not text. pdftract must recognize photograph placeholders by their aspect ratio and position within the form layout and annotate them as `photograph_placeholder` in extraction metadata rather than attempting to interpret the image content as text. Checkbox responses — for questions about criminal history, dual nationality, or prior visa refusals — follow the same extraction rules as immigration forms: preserve state and parent question association.

---

## Government Procurement Forms (SF-86, SF-1449, DD-254)

Federal procurement and security clearance forms are dense structured tables. The SF-86 (Questionnaire for National Security Positions) contains over 120 pages of field tables with text, checkbox, and date inputs. The SF-1449 (Solicitation/Contract/Order) and DD-254 (Contract Security Classification Specification) similarly use tabular grid layouts where cell boundaries delineate field scope.

pdftract must use cell boundary geometry — detected from vector path segments or whitespace analysis — to associate field values with their labels correctly. In multi-column procurement forms, naive reading-order extraction produces garbled output by interleaving column A and column B content. Geometric table detection must take precedence over Unicode reading order for these forms.

Classification markings in headers and footers appear as boldface centered text on procurement forms (discussed further below). Certification blocks — contractor signature, date, and Contracting Officer Representative fields — should be preserved with their structural context.

---

## Court Filing Cover Sheets and Civil Cover Sheets (JS-44)

The JS-44 civil cover sheet filed with federal district courts contains a checkbox array for nature-of-suit codes, jurisdiction basis, and origin of the action. Each checkbox corresponds to a category code (for example, 422 for Bankruptcy Appeal, 110 for Insurance). pdftract must extract both the checkbox state and the numeric code, since the code — not the label text — is the authoritative data element consumed by court filing systems.

Party information fields (plaintiff name, defendant name, attorneys of record, county of residence) are typically AcroForm fields or typed-text overlays on a pre-printed form background. Case category codes in the nature-of-suit section appear in dense two-column checkbox arrays; geometric layout analysis ensures the correct code is associated with each checked entry.

---

## Government-Generated Flat PDF Reports

Not all government PDFs are interactive forms. FedBizOpps and SAM.gov opportunity PDFs, FOIA response packets, and regulatory docket documents published by agencies such as the EPA, SEC, or FTC are typically flat PDFs generated by report engines or document management systems. These contain no AcroForm fields whatsoever. The entire content is vector text organized in paragraphs, tables, and headers.

pdftract's extraction path for these documents is straightforward text and table extraction without any field-detection logic. However, FOIA response packets often combine generated cover letters (vector text) with scanned exhibit pages (rasterized images), requiring pdftract to handle mixed-mode PDFs on a per-page basis — applying OCR only to pages where no selectable text layer exists, and using the vector text layer directly on pages where it is present.

---

## Certificate and License PDFs

Professional licenses, birth certificate printouts, deeds, and government-issued certificates present a different extraction challenge. The semantic content — licensee name, license number, issuance date, expiration date, issuing authority — is encoded as vector text and is fully extractable. The visual complexity of these documents comes from decorative and security elements that are not text.

Embossed seals appear as rasterized images embedded in the PDF. Security paper backgrounds — colored fiber patterns, guilloche designs, watermarks — are either embedded images or vector graphic layers. pdftract should ignore image content that does not contain extractable text and should not attempt to OCR decorative background layers. The structured text fields on these certificates remain in the vector text layer and can be extracted directly without any image processing.

---

## Government Scan and OCR PDFs

A substantial fraction of government documents available through FOIA releases, court dockets, and agency archives are legacy scans. These were typically scanned at 200 to 300 DPI on flatbed or document-feed scanners and subsequently OCR'd, sometimes with the OCR text embedded as an invisible layer over the raster image. Quality varies significantly by agency and era.

pdftract applies its standard OCR correction pipeline to these documents: character confidence scoring, dictionary-based correction for common OCR errors (rn/m substitution, 0/O confusion, l/1 confusion), and layout reconstruction to recover paragraph and column structure from line-coordinate clustering.

Government scan artifacts require specific handling. Hole punches along the left margin of three-ring-binder documents create dark circular regions that OCR engines frequently misinterpret as characters. pdftract detects circular high-contrast regions within the left margin zone (approximately 0.5 inches from the left edge) and masks them before OCR. Stamps — RECEIVED, APPROVED, CLASSIFIED, VOID — are typically rotated text images overlaid on the document. pdftract detects high-contrast rectangular or free-form rotated overlays and processes them separately, flagging detected stamp regions as `overlay_stamp` in per-page metadata with the extracted text if legible.

---

## Barcodes in Government Forms

PDF417, QR, and Code 39/128 barcodes appear as rasterized images within government PDFs. The barcode payload itself — whether it encodes form data, a tracking number, or an application identifier — is not text-extractable by pdftract. Attempting to decode barcode pixel data as text produces garbage output.

The correct behavior is detection and flagging. When pdftract identifies an image region that matches barcode structural characteristics (high-frequency vertical striping for 1D barcodes, square matrix patterns for 2D barcodes), it records a `barcode_detected` annotation in the extraction output with the bounding box of the barcode image in page coordinates. Human-readable text printed above or below the barcode — a form number, a confirmation code, an applicant identifier — is in the vector or OCR text layer and is extracted normally. callers that need barcode payloads must route those image regions to a dedicated barcode decoder outside pdftract.

---

## Classification and Handling Markings

Government documents subject to information controls carry standardized marking strings in page headers and footers. Common markings include UNCLASSIFIED, CONTROLLED UNCLASSIFIED INFORMATION (CUI), FOR OFFICIAL USE ONLY (FOUO), SENSITIVE BUT UNCLASSIFIED (SBU), and PRIVACY ACT PROTECTED. Classified documents add SECRET, TOP SECRET, and compartment designators, though these are uncommon in documents available through public channels.

These markings appear as text — typically bold, centered, uppercase — in repeating header and footer positions across all pages of the document. pdftract extracts them as normal text but additionally inspects the set of recognized marking strings and records any matches in a `handling_markings` array in the document-level extraction metadata. This allows callers to surface classification status programmatically without parsing free-form header text.

The marking strings themselves are not redacted or suppressed. A document marked CUI may have substantive content redacted (appearing as black rectangles or blank space), but the CUI marking and any handling instructions (such as CUI//PRVCY or CUI//LAW) are always present in the text layer and must be preserved in extraction output.

---

## Summary

Government form PDFs demand that pdftract correctly navigate AcroForm fields with semantic labels, multi-part section hierarchies, checkbox state preservation, mixed scan-and-vector page composition, barcode detection without payload decoding, geometry-driven table extraction, and classification marking identification. No single extraction strategy covers this space. pdftract's layered approach — AcroForm field extraction, vector text extraction, geometric layout analysis, per-page mode detection, and OCR with artifact correction — provides the coverage necessary to produce complete, accurate, and structurally faithful text from the full range of government-origin PDFs.
