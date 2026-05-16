# PDF Encryption and Security

## Purpose

This document describes how `pdftract` detects, decrypts, and processes encrypted PDFs to enable text extraction. It covers the Standard security handler across all revisions, key derivation algorithms, object-level decryption, permission flag semantics, and the implementation approach using the RustCrypto ecosystem.

---

## 1. Encryption Detection

Before parsing any content objects, `pdftract` must inspect the trailer dictionary for the `/Encrypt` entry. The trailer is located by following the `startxref` offset, and its dictionary is parsed before any cross-reference resolution. If the trailer contains an `/Encrypt` key, the file is encrypted.

The `/Encrypt` value is an indirect reference to the encryption dictionary. The `Filter` name within that dictionary identifies the security handler:

- `/Standard` — password-based encryption (all PDF versions)
- `/Adobe.PubSec` — certificate-based encryption using public-key cryptography

If `/Encrypt` is present and no password has been supplied by the caller, `pdftract` must fail fast with `EncryptionError::PasswordRequired` before attempting object decoding. Proceeding without decryption produces garbage text and silent data corruption.

Cross-reference streams (PDF 1.5+) are themselves not encrypted even in encrypted documents. The trailer and cross-reference data remain plaintext so the reader can locate the encryption dictionary before decrypting anything else.

---

## 2. Standard Security Handler

The Standard security handler (`/Filter /Standard`) is the ubiquitous password-based scheme. Its revision history maps directly to the cryptographic strength available:

| V | R  | Algorithm            | Key size | PDF version |
|---|----|----------------------|----------|-------------|
| 1 | 2  | RC4                  | 40-bit   | 1.1–1.3     |
| 2 | 3  | RC4                  | 128-bit  | 1.4         |
| 3 | 4  | RC4 or AES-128       | 128-bit  | 1.5         |
| 4 | 5  | AES-256 (SHA-256)    | 256-bit  | 1.7 ext3    |
| 5 | 6  | AES-256 (SHA-512)    | 256-bit  | PDF 2.0     |

The `/Encrypt` dictionary contains:

- `Filter` — `/Standard`
- `SubFilter` — optional; specifies a more specific handler
- `V` — encryption algorithm version (integer)
- `R` — revision of the Standard handler
- `O` (32 bytes for R2–R4; 48 bytes for R5/R6) — owner password verifier or encrypted intermediate key
- `U` (32 bytes for R2–R4; 48 bytes for R5/R6) — user password verifier or encrypted intermediate key
- `OE` / `UE` (32 bytes; R5/R6 only) — file encryption key encrypted under the owner/user intermediate key
- `P` — signed 32-bit integer encoding permission flags
- `Length` — key length in bits (R3/R4; default 40 for R2)
- `EncryptMetadata` — boolean; if false, the XMP metadata stream is not encrypted (default true)
- `CF` / `StmF` / `StrF` — crypt filter table and per-stream/string filter names (R4+)

---

## 3. Key Derivation — R2, R3, R4

The file encryption key is derived via MD5. The algorithm follows these steps precisely:

1. **Password padding.** Take the user-supplied password (up to 32 bytes, zero-padded or truncated), then append bytes from the canonical 32-byte padding string defined in the specification until the combined length reaches 32 bytes.

2. **Hash construction.** Initialize MD5 and feed it:
   - The 32-byte padded password
   - The 32-byte `O` entry from the encryption dictionary
   - The 4-byte `P` value in little-endian order
   - The first 16 bytes of the file identifier (the first element of the `/ID` array in the trailer)
   - If `R >= 4` and `EncryptMetadata` is false, the 4-byte sequence `0xFF 0xFF 0xFF 0xFF`

3. **Iteration (R3+).** For revisions 3 and above, repeat the MD5 hash 50 times, each time hashing the previous result, restricted to `n` bytes where `n = Length / 8`.

4. **Truncation.** The file encryption key is the first `n` bytes of the final MD5 output. For R2, `n = 5` (40-bit key). For R3/R4, `n = Length / 8` (up to 16 bytes).

**User password verification.** To verify that the supplied password is correct for R2, RC4-encrypt the 32-byte padding string with the derived key and compare against the first 16 bytes of the `U` entry. For R3/R4, additionally hash the padding string with the file identifier, encrypt with the file key and then encrypt the result 19 more times with modified keys (key bytes XORed with the iteration counter 1–19), and compare against the first 16 bytes of `U`.

**Owner password.** The owner password encrypts the user password and stores it in the `O` entry. To check the owner password: derive an MD5-based key from the padded owner password (same padding step, no O/P/ID involvement), RC4-decrypt the `O` entry to recover the user password, then run the user key derivation with the recovered password.

---

## 4. Key Derivation — R5 and R6 (PDF 2.0)

R5 and R6 replace MD5 with SHA-based hashing and use a two-stage key structure. The file encryption key is a random 256-bit value stored encrypted inside the `OE` and `UE` entries.

**R5 (deprecated).** The 48-byte `U` entry consists of a 32-byte hash (SHA-256 of the padded password concatenated with an 8-byte validation salt and an 8-byte key salt) followed by the 16-byte validation salt and key salt. To verify the user password: compute SHA-256 of `password || validation_salt` and compare against the first 32 bytes of `U`. To derive the intermediate key: compute SHA-256 of `password || key_salt`. Decrypt `UE` using AES-256-CBC with this intermediate key (zero IV) to obtain the 32-byte file encryption key. R5 is deprecated because its single SHA-256 round provides insufficient password hashing strength and is vulnerable to GPU-accelerated brute force.

**R6 (PDF 2.0).** R6 uses a more complex iterative hash function. The intermediate key computation replaces the single SHA-256 with an adaptive loop: starting with SHA-256, it repeatedly hashes a sequence of `password || round_input || user_key_salt` (using SHA-256, SHA-384, or SHA-512 based on the last byte of the previous hash output modulo 3), continuing until a termination condition based on the last byte of the hash is met. This makes the function significantly more resistant to precomputation. The structure of `U`, `UE`, `O`, and `OE` is otherwise analogous to R5.

The owner variants (`O`, `OE`) additionally include the 48-byte `U` value in the hash input, binding the owner key to the specific user key entry.

---

## 5. Object-Level Decryption

**R2–R4.** Each encrypted string and stream body uses a per-object key derived from the file encryption key. The derivation:

1. Take the file encryption key bytes.
2. Append the 3 low-order bytes of the object number in little-endian order.
3. Append the 2 low-order bytes of the generation number in little-endian order.
4. For AES streams (R4 with `StmF` specifying AES), additionally append the 4-byte sequence `0x73 0x41 0x6C 0x54` ("sAlT").
5. Compute MD5 of this concatenation and take the first `min(n + 5, 16)` bytes as the per-object key.

RC4 is a stateless stream cipher: apply it directly to the ciphertext. AES-128 uses CBC mode with a 16-byte random IV prepended to the ciphertext; strip the IV before decryption.

**R5/R6.** No per-object key derivation. AES-256-CBC is used for all strings and streams with a 16-byte random IV prepended to each individual ciphertext. The same file encryption key applies to every object.

**Crypt filter opt-out.** A stream may include a `Crypt` filter with `/Name /Identity` in its filter pipeline to declare that it is not encrypted. This mechanism is used for streams that must be readable before the encryption context is established. `pdftract` must check for this before applying decryption to any stream.

**Cross-reference streams** are never encrypted, regardless of handler or revision.

---

## 6. Permission Flags

The `P` entry is a signed 32-bit integer. Bits are numbered from 1 (LSB). Bits 1–2 are reserved and must be zero in the key derivation. The semantically significant bits:

| Bit | Meaning                                  |
|-----|------------------------------------------|
| 3   | Print the document                       |
| 4   | Modify the document                      |
| 5   | Copy or extract text and graphics        |
| 6   | Add or modify annotations and forms      |
| 9   | Fill interactive form fields             |
| 10  | Extract text for accessibility           |
| 11  | Assemble the document                    |
| 12  | Print at high fidelity                   |

A bit value of 0 means the permission is denied. Bits not listed are reserved.

**Bit 5 (extract text)** is the primary flag governing `pdftract`'s core operation. By default, `pdftract` should respect this flag and return `EncryptionError::ExtractionNotPermitted` if bit 5 is clear. Bit 10 (extract for accessibility) provides an alternative grant; if bit 10 is set and bit 5 is clear, accessibility-mode extraction may be allowed. `pdftract` should expose an `allow_accessibility_extraction: bool` option in `ExtractionConfig` to give callers explicit control over this behavior, particularly for screen readers and assistive tooling.

---

## 7. Public-Key Encryption

When `Filter` is `/Adobe.PubSec`, the file is encrypted for one or more specific certificate holders. The encryption dictionary contains a `Recipients` array; each entry is a CMS `EnvelopedData` structure containing the file encryption key wrapped for a specific recipient's X.509 certificate using the recipient's RSA public key.

To decrypt, the caller supplies their private key. `pdftract` iterates the recipient list, attempts decryption of each `EnvelopedData` blob, and uses the unwrapped key as the file encryption key. The actual content encryption algorithm (RC4 or AES at various key sizes) is specified within the CMS structure.

Public-key encryption is not required for the initial implementation. The detection path must be present: if `Filter` is `/Adobe.PubSec`, `pdftract` returns `EncryptionError::UnsupportedSecurityHandler` with a descriptive message identifying the handler name.

---

## 8. Encrypted Metadata

When `EncryptMetadata` is `false` (not the default), the XMP metadata stream (if present as an indirect object) is excluded from encryption. Its stream data can be decoded without a password. This is relevant for applications that need document metadata (title, author, creation date) without performing full content decryption.

The `/Info` dictionary, however, is always encrypted in Standard-handler documents. Its string values (title, subject, keywords, etc.) require the file encryption key to decode. `pdftract` must apply per-object decryption to `/Info` strings whenever the file is encrypted, regardless of `EncryptMetadata`.

---

## 9. Implementation Approach

The RustCrypto ecosystem provides all necessary primitives:

- `md-5` — MD5 for R2–R4 key derivation and per-object key computation
- `sha2` — SHA-256/384/512 for R5/R6 intermediate key derivation
- `rc4` — RC4 stream cipher for R2/R3/R4 string and stream decryption
- `aes` + `cbc` — AES-128-CBC (R4) and AES-256-CBC (R5/R6) decryption
- `cbc` — CBC mode wrapper (from `cipher` crate)

Structure the decryptor as a `Decryptor` type that wraps the parsed encryption dictionary and holds the derived file encryption key. The object parser passes raw ciphertext bytes through `Decryptor::decrypt_string(obj_num, gen_num, ciphertext)` and `Decryptor::decrypt_stream(obj_num, gen_num, ciphertext)` before returning parsed objects to higher layers. This keeps decryption transparent to the text extraction layer.

Cache the file encryption key on the `Decryptor` after first derivation. Per-object keys for R2–R4 are cheap to compute (one MD5 per object) and need not be cached; computing them inline avoids the memory overhead of a per-object map.

---

## 10. Error Handling

Define an `EncryptionError` enum in the public API:

```rust
pub enum EncryptionError {
    /// Encrypted document; no password was provided.
    PasswordRequired,
    /// The supplied password did not match the document's owner or user password.
    WrongPassword,
    /// The encryption dictionary is missing required entries or is structurally invalid.
    InvalidEncryptionDictionary(String),
    /// The revision or V value is not supported.
    UnsupportedRevision { v: u8, r: u8 },
    /// The security handler is not supported (e.g., Adobe.PubSec).
    UnsupportedSecurityHandler(String),
    /// The permission flags deny text extraction.
    ExtractionNotPermitted,
    /// Decryption of a specific object failed (truncated ciphertext, bad IV, etc.).
    ObjectDecryptionFailed { obj_num: u32, gen_num: u16 },
}
```

**Wrong password** is detected at key derivation time by comparing the derived key's output against the `U` entry as described in Section 3. Return `WrongPassword` immediately rather than allowing garbage decryption to propagate.

**Corrupted encryption dictionary** — missing `V`, `R`, `O`, `U`, `P`, or `/ID` — should return `InvalidEncryptionDictionary` with a message identifying the missing field.

**Unsupported revision** — any `V`/`R` combination outside the table in Section 2 — returns `UnsupportedRevision`. This handles future revisions gracefully without panicking.

**Partially encrypted files** are rare but real: some producers mark the file as encrypted while leaving certain streams unencrypted. If decryption of an individual object's ciphertext fails (e.g., AES block size mismatch), return `ObjectDecryptionFailed` and continue processing other objects, allowing partial text extraction with a warning in the result set.
