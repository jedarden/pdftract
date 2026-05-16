# Medical and Scientific PDF Extraction Patterns

## Overview

Medical and scientific PDFs represent some of the most structurally demanding documents that a text extraction engine will encounter. They combine dense tabular data, specialized notation, hierarchical section numbering, regulatory formatting conventions, and typographic symbols that are frequently mangled by naive extraction approaches. This document describes the patterns pdftract must handle correctly to produce reliable, readable output from clinical, pharmaceutical, and academic scientific documents.

---

## 1. Clinical Trial Reports (ICH E3 Format)

Clinical study reports following the ICH E3 guideline present a layered section hierarchy: numbered top-level sections (1. Synopsis, 2. Table of Contents, 3. Introduction, through 16. Appendices) with multi-level subsections such as 9.4.1.1 and deeper. pdftract must preserve this numbering scheme exactly — dropped or merged numbers corrupt the logical outline that downstream systems rely on for navigation.

The protocol synopsis table at the front of an ICH E3 report is a structured two-column layout: parameter name on the left, value on the right. Rows cover study title, protocol number, phase, objectives, investigational product, dose regimen, patient population, and statistical design. Because these tables are sometimes rendered as grid-lined tables and sometimes as borderless aligned text, pdftract must detect both forms and emit them as coherent key-value pairs rather than interleaved character streams.

Patient flow is typically represented as a CONSORT diagram — a flowchart image showing enrolled, randomized, allocated, lost to follow-up, and analyzed counts. The diagram itself is a vector graphic or rasterized image and its internal numbers are not extractable from the image data. However, CONSORT diagrams are always accompanied by a figure caption (e.g., "Figure 2. Patient disposition") that pdftract must pair with the figure block. The caption text, including the embedded counts if they appear in text form beneath the figure, must be extracted and associated with the figure reference.

Adverse event tables are dense, multi-column structures with system organ class groupings, preferred terms, and per-arm count columns. Column headers often span multiple rows, and the system organ class rows use bold or indented formatting to distinguish them from the preferred term rows underneath. pdftract must preserve row-level indentation signals to allow downstream consumers to reconstruct the hierarchy without ambiguity.

---

## 2. Medical Journal Articles (IMRaD Format)

The IMRaD structure — Abstract, Introduction, Methods, Results, Discussion — is the dominant organizational pattern for biomedical journal articles. Section headers may be formatted as bold inline text, as visually distinct heading blocks, or, in two-column layouts, as text spanning the full page width while body text flows in columns. pdftract must detect IMRaD section boundaries regardless of whether they carry PDF structural tags, and emit them as block-level labels so that downstream processing can distinguish Methods text from Results text without manual parsing.

Structured abstracts add a sublevel: Background, Objective, Methods, Results, Conclusions appear as bold inline labels within a single abstract block. These must be captured as labeled sub-blocks rather than collapsed into undifferentiated paragraph text.

Two-column journal layouts require careful column ordering. Text in the left column must be read before text in the right column at equivalent vertical positions, and figure captions placed between columns or spanning both must be associated with the correct figure rather than inserted mid-sentence into the adjacent body text stream.

---

## 3. Drug Labeling PDFs (FDA Prescribing Information)

FDA-format package inserts follow a mandated section structure numbered 1 through 17: Indications and Usage (1), Dosage and Administration (2), Dosage Forms and Strengths (3), Contraindications (4), Warnings and Precautions (5), Adverse Reactions (6), Drug Interactions (7), Use in Specific Populations (8), Drug Abuse and Dependence (9), Overdosage (10), Description (11), Clinical Pharmacology (12), Nonclinical Toxicology (13), Clinical Studies (14), References (15), How Supplied/Storage and Handling (16), Patient Counseling Information (17). pdftract must recognize and tag these numbered headings to enable downstream systems to locate, for example, all Warnings and Precautions content across a batch of package inserts.

The Highlights of Prescribing Information box appears at the top of the label as a visually distinct shaded or bordered block, formatted in a condensed multi-column layout that summarizes the most critical sections. Extraction must preserve the box as a distinct block separate from the full prescribing information body, because regulatory systems treat Highlights as a standalone artifact.

Boxed Warnings — colloquially Black Box Warnings — are surrounded by a heavy border and typically set in bold text. They represent the highest severity safety signal in US labeling. pdftract must detect boxed warning blocks and apply a safety-critical flag to the extracted text so that downstream consumers can surface these warnings without parsing the full document. Detection heuristics include: a bordered rectangle enclosing bold text near the document top, the phrase "WARNING" or "BOXED WARNING" as a heading within the bordered region, and positioning before section 1 of the main label body.

---

## 4. Lab Reports and Pathology Reports

Laboratory and pathology report PDFs are typically generated from laboratory information systems (LIS) and follow a predictable structure: a patient metadata header containing name, date of birth, collection date/time, ordering provider, specimen type, and accession number; followed by a results table with columns for test name, result value, reference range, units, and an abnormality flag (H for high, L for low, C for critical).

pdftract must extract these tables column-by-column rather than row-by-row to prevent units from being concatenated with result values or flags from drifting to incorrect rows. Reference ranges expressed as "3.5–5.0" must survive extraction with the en dash intact rather than being converted to a hyphen or dropped entirely. Flag values (H, L, HH, LL, C, A) are short and may be misread as part of the unit column if column alignment is not honored.

Patient metadata headers warrant special handling: the fields appear in varied multi-column layouts and sometimes as label-colon-value inline text. pdftract must recognize the header region and extract it as structured metadata distinct from result rows.

---

## 5. Scientific Paper Extraction: DOIs, Citations, and Author Affiliations

Reference sections in scientific papers follow numbered or author-date citation formats. DOIs appear as "https://doi.org/10.xxxx/..." or as bare "DOI: 10.xxxx/..." strings. pdftract must extract DOIs as intact strings; line-wrapping within a DOI is a frequent extraction failure point because a hyphenated break inside the DOI path produces an invalid identifier. pdftract must detect mid-DOI line breaks and rejoin them.

Author affiliation blocks link author names to institutions via superscript numerals or symbols. The author line appears at the top of the article, with each author followed by one or more superscript markers (¹, ², *), and the corresponding affiliations appear in smaller text below. pdftract must associate superscript markers with their author names and with their expanded affiliation strings, preserving the many-to-many relationship in the extraction output.

ORCID iDs appear as 16-digit strings in the format 0000-0002-1825-0097, often following the ORCID logo (an image) or the label "ORCID:". pdftract must extract these as text, treating the image as non-extractable but capturing the adjacent identifier string.

---

## 6. Chemical Structures in PDFs

Chemical structure diagrams are rendered as vector graphics or embedded images. The atoms and bonds that constitute a structural formula are not encoded as text characters in the PDF content stream; they are geometric drawing operations. pdftract must not attempt to interpret structure diagrams as text and must instead mark the bounding region as a figure placeholder.

The extractable chemical identity information appears in surrounding text as IUPAC systematic names (e.g., "(2S)-2-amino-3-(4-hydroxyphenyl)propanoic acid"), CAS registry numbers (e.g., CAS 60-18-4), and occasionally as InChI strings (e.g., "InChI=1S/..."). InChI strings are long and may wrap across lines; pdftract must detect them by prefix and rejoin wrapped segments. SMILES strings may also appear in supplementary tables. All of these are plain text and must be extracted verbatim.

---

## 7. Statistical Notation

Medical papers are dense with statistical notation that is vulnerable to extraction errors. Confidence intervals appear as "95% CI: 1.23–4.56" or "95% CI [1.23, 4.56]"; the en dash or em dash separating the bounds must survive as the correct Unicode character rather than being dropped or replaced with an ASCII hyphen. p-values are expressed with the less-than symbol: "p < 0.001"; the `<` character must not be interpreted as an XML/HTML tag boundary. Similarly, ≤ and ≥ are used in threshold expressions and must be extracted as their Unicode code points (U+2264, U+2265).

Hazard ratios and odds ratios appear with confidence intervals: "HR 0.72 (95% CI 0.58–0.89)" — the parenthetical CI must be kept on the same logical line as the ratio. The multiplication sign × (U+00D7) appears in cell count expressions and must be preserved; the common failure is replacement with the letter x.

---

## 8. Units and Measurement Notation

SI unit notation in medical PDFs includes the micro prefix µ (U+00B5 or U+03BC), which is distinct from the ASCII letter u and must not be substituted for it. Degree symbols ° (U+00B0) appear in temperature measurements. These are frequently lost when PDFs are produced from fonts that map these characters to non-standard code points.

Subscript characters in chemical formulas — H₂O, CO₂, CaCO₃ — may be encoded as actual Unicode subscript digits (U+2082, U+2082) or as font-size-reduced characters positioned below the baseline. pdftract must normalize both representations to Unicode subscript characters or, where appropriate, to a plaintext markup form, rather than omitting them or rendering them at the same baseline as surrounding text.

---

## 9. Supplementary Material References

Authors routinely direct readers to supplementary material that exists as a separate file. Phrases such as "Supplementary Table S1", "Supplementary Figure S2", "see online supplementary appendix", and "eTable 3 in the Supplement" appear as inline text and must be extracted verbatim. These cross-references are textually meaningful even when the supplement is not present, because they indicate that a referenced data artifact exists. pdftract must not strip these references as unreachable hyperlink targets.

---

## 10. Regulatory Submission PDFs (NDA/BLA eCTD Format)

Electronic Common Technical Document (eCTD) submissions are organized into five modules: Module 1 (Administrative), Module 2 (Summaries), Module 3 (Quality), Module 4 (Nonclinical Study Reports), Module 5 (Clinical Study Reports). Individual PDF files within an eCTD submission are navigated via a bookmark tree that mirrors the eCTD section hierarchy (e.g., m5/5.3/5.3.5.1/). pdftract must extract the PDF bookmark tree and emit it as a structured outline alongside the document text, because eCTD reviewers navigate primarily by section number.

Form FDA 356h is the cover sheet for NDA and BLA submissions. It is a structured form with labeled fields: applicant name, address, NDA/BLA number, date of submission, proposed proprietary name, established name, pharmacological class, dosage form, route of administration, and a checklist of attached documents. pdftract must recognize form field regions and extract label-value pairs from them, preserving field boundaries even when the form is rendered as a static PDF image of a filled form rather than an interactive AcroForm.

The Module 2 summaries — Quality Overall Summary, Nonclinical Overview, Clinical Overview, Clinical Summary — are narrative documents that reference section numbers in Modules 3–5. These cross-references (e.g., "see section 5.3.5.1") must be preserved as text so that downstream indexing can reconstruct the cross-module citation graph.

---

## Summary

Correct extraction from medical and scientific PDFs requires pdftract to handle section hierarchy preservation, multi-column layout ordering, special Unicode characters, table column integrity, figure-caption association, form field recognition, and bookmark tree extraction as first-class concerns. Failures in any of these areas produce output that is either unreadable to human reviewers or unparseable by downstream regulatory and clinical data systems. The patterns described here define the minimum correctness bar for pdftract to be a reliable tool in pharmaceutical, clinical research, and academic scientific workflows.
