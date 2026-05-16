# Digital Signatures, Certification, and Document Integrity in PDFs

**Project:** pdftract — Rust PDF text extraction library
**Scope:** Signature field types, cryptographic metadata extraction, incremental update semantics, and integrity context for extraction confidence

---

## 1. Signature Field Types and Permitted Modifications

PDF defines three signature field subtypes, each with different implications for how much of a document's content can be treated as the original signed payload.

An **approval signature** means a named entity approved the document at signing time. It does not restrict subsequent modifications; later editors may append content via incremental updates, and multiple approval signatures may appear in one document, each covering the cumulative byte range up to the moment that signature was applied.

A **certification signature** (DocMDP) is always the first signature applied. It is registered at the catalog level through a `/Perms` dictionary with a `/DocMDP` reference, and its `/Reference` array contains a transform dictionary with `/TransformMethod /DocMDP` and a `/P` entry encoding the permissions level. Any modification exceeding the permitted level technically invalidates the certification. pdftract extracts the certification signer name and permissions level and annotates extracted text regions accordingly.

A **usage rights signature** (UR3) enables otherwise-restricted reader features such as saving filled form data. UR3 signatures yield signer identity and certificate metadata for extraction but do not constrain which text is readable.

---

## 2. The /Sig Dictionary and Extractable Fields

A signature field's value object is a signature dictionary (`/Type /Sig`). Its entries are the primary source of structured metadata pdftract surfaces.

**`ByteRange`** is an array of four integers `[offset1, length1, offset2, length2]` defining the byte spans covered by the cryptographic hash. Everything in the file except the Contents placeholder between those two ranges is hashed. pdftract uses ByteRange to distinguish signed bytes from post-signature incremental additions.

**`Contents`** holds the cryptographic payload as a hexadecimal DER blob — a PKCS#7 (CMS) SignedData structure. pdftract parses this to retrieve the embedded signer certificate, the signing time from authenticated attributes, and any embedded timestamp token. It yields no extractable prose but is the source of all certificate metadata.

**`SubFilter`** identifies the format: `adbe.pkcs7.detached` is the most common, embedding a full SignedData with certificate and optional RFC 3161 timestamp; `adbe.pkcs7.sha1` is a legacy SHA-1 variant; `ETSI.CAdES.detached` is used in European qualified signature workflows with stricter attribute requirements; `ETSI.RFC3161` contains a bare timestamp token rather than a signer signature, asserting document existence at a moment in time.

The signature dictionary may also carry four text strings pdftract extracts directly: **`Name`** (signer name as entered in the UI), **`M`** (signing time in PDF date format as claimed by the signing application), **`Reason`** (free-text explanation), and **`Location`** (signer's reported location). These are caller-supplied and unverified; pdftract marks them as `claimed` in the output schema.

---

## 3. Signature Appearance Streams

A signature field widget annotation may have an `/AP` dictionary whose normal appearance (`/N`) is a Form XObject — the visual stamp rendered on the page, typically showing the signer's name, date, and reason text.

From pdftract's perspective, this appearance stream is a standard content stream parsed identically to any other Form XObject. Text-showing operators (`Tj`, `TJ`, `'`, `"`) position strings in whatever font the XObject's `Resources` dictionary declares. pdftract resolves character codes through font encoding and ToUnicode maps exactly as it does for page content. The extracted appearance text is often more composed than the raw dictionary fields — signing applications format date and reason into readable sentences — and pdftract includes it alongside the structured metadata in the signature record.

---

## 4. DocMDP Permissions and Extraction Confidence

The `/P` integer in the DocMDP transform dictionary takes three values with specific meaning for post-certification modifications: **1** permits no changes; **2** permits form fills, widget annotation changes, and new approval signatures; **3** additionally permits page and annotation insertions and deletions.

These levels have direct implications for extraction confidence. Content within the original certified byte range was hash-covered by the certification. Content from incremental updates may or may not be within the scope the certifier permitted. pdftract surfaces the DocMDP level so consuming applications can decide how much trust to place in each extracted text region.

---

## 5. Incremental Updates After Signing

When a user fills a form or adds an approval signature after an earlier signature, the PDF receives an incremental update: an additional body appended after the previous `%%EOF` marker, with its own cross-reference data and new or modified objects. pdftract must process all incremental updates to extract complete document content, chaining cross-reference sections from the latest trailer `/Prev` offset back to the original, building an object table where each number resolves to the most recently written version.

Any byte that falls beyond the union of a given signature's ByteRange spans was added after that signature was computed. pdftract records for each text region which signatures' byte ranges cover it — regions covered by at least one signature carry an associated integrity note; regions added after all signatures lack any cryptographic cover.

---

## 6. LTV Signatures and the DSS Dictionary

Long Term Validation (LTV) embeds certificate revocation data into the PDF via the Document Security Store (`/DSS` at the catalog level). The DSS dictionary contains arrays of binary DER-encoded blobs: **`OCSPs`** (OCSP responses), **`CRLs`** (Certificate Revocation Lists), and **`Certs`** (chain certificates). A `/VRI` sub-dictionary maps signature hashes to their associated validation data.

None of this is prose text. pdftract detects the presence of a populated DSS dictionary and reports a `ltv_present` boolean plus counts of embedded OCSP responses and CRLs. The presence of LTV data indicates that revocation status was confirmed as good at some point after signing — meaningful context for how much confidence to attach to the extracted content.

---

## 7. Timestamp Tokens and Reliable Signing Time

An **embedded timestamp** appears as an unsigned attribute (`id-aa-signatureTimeStampToken`) within the PKCS#7 SignedData in the signature's Contents field. pdftract parses the `unsignedAttrs` of the first SignerInfo and, if present, extracts the TSA's `genTime` — a GeneralizedTime value that is cryptographically guaranteed by the TSA's own signature.

A **document timestamp** uses `SubFilter ETSI.RFC3161` and contains a bare RFC 3161 TimeStampToken as the Contents blob. These are commonly added by archival workflows to establish a trusted time anchor.

The TSA-asserted `genTime` is significantly more reliable than the `/M` entry in the signature dictionary, which is set by the signing application and can be misconfigured or forged. pdftract's hierarchy for the `signing_time` output field is: RFC 3161 `genTime` if present, annotated `timestamp_token`; otherwise `/M`, annotated `claimed`.

---

## 8. Certificate Information Extraction

The signer certificate embedded in the PKCS#7 Contents field is a DER-encoded X.509 certificate. pdftract parses the TBSCertificate to extract the following text fields: **Subject DN** (including `commonName`, `organizationName`, `organizationalUnitName`, `countryName`, `emailAddress`); **Issuer DN** (same RDN structure, identifying the Certificate Authority); **Serial number** (rendered as colon-separated hex); and the **validity period** (`notBefore` / `notAfter`). If the TSA-extracted signing time falls outside the certificate's validity period, pdftract flags the record with `certificate_validity_issue: true`. Chain certificates from the PKCS#7 `certificates` field are parsed for subject and issuer DNs but not deeply surfaced; pdftract records chain depth and root issuer name.

---

## 9. Signature Field Widget Annotations

Signature fields manifest on the page as widget annotations with `/Rect` (bounding rectangle in page user space), `/P` (page reference), and `/AP` (appearance dictionary). The widget `/F` flags field includes bit 2 (`Hidden`) and bit 7 (`NoView`); if either is set, the signature has no visible representation and pdftract records `visible: false`. A non-visible signature remains cryptographically valid but contributes no appearance text for extraction. The bounding rectangle is recorded as `[x1, y1, x2, y2]` so consuming applications can associate signature metadata with a specific page region.

---

## 10. Output Schema for Signature Metadata

pdftract emits signature metadata as a `signatures` array at the top level of its JSON output, parallel to `pages` and `metadata`. Each entry is a `SignatureRecord` with the following fields:

| Field | Type | Source |
|-------|------|--------|
| `field_name` | string | Field `/T` |
| `signature_type` | enum | SubFilter / transform method |
| `signer_name` | string | `/Name` (claimed) |
| `signer_common_name` | string | Certificate CN (cryptographic) |
| `signer_organization` | string | Certificate O (cryptographic) |
| `issuer_dn` | string | Certificate issuer RFC 4514 string |
| `serial_number` | string | Hex with colon separators |
| `cert_not_before` | RFC 3339 | Certificate notBefore |
| `cert_not_after` | RFC 3339 | Certificate notAfter |
| `signing_time` | RFC 3339 | TSA genTime or `/M` |
| `signing_time_source` | enum | `timestamp_token` or `claimed` |
| `reason` | string\|null | `/Reason` (claimed) |
| `location` | string\|null | `/Location` (claimed) |
| `mdp_permissions` | int\|null | DocMDP `/P`; null if not certification |
| `byte_range` | [u64; 4] | `/ByteRange` |
| `ltv_present` | bool | DSS dictionary presence |
| `ocsp_count` | integer | DSS OCSPs array length |
| `crl_count` | integer | DSS CRLs array length |
| `appearance_text` | string\|null | Extracted from `/AP/N` XObject |
| `visible` | bool | Widget annotation flags |
| `page` | integer | 1-based page number |
| `rect` | [f32; 4] | `[x1, y1, x2, y2]` user space |
| `certificate_validity_issue` | bool | Computed from signing_time vs. cert window |

The `signing_time_source` field is essential for calibrating trust in extracted content. A `timestamp_token` source means a third-party TSA has cryptographically bound the document hash to a specific moment; a `claimed` source means the signing application reported the time. For extraction confidence, regions whose only covering signature has `signing_time_source: claimed` and no LTV data carry softer provenance than regions covered by a fully LTV-enabled, TSA-timestamped certification.

The `byte_range` field enables the consuming application to cross-reference each signature's coverage against the page-level text regions in pdftract's `pages` output, determining with byte-level precision which extracted strings were present when each signature was applied and which were added after the fact.
