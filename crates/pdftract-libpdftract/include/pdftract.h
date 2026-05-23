/* Copyright 2026 Jed Cabanino. MIT OR Apache-2.0 */

#ifndef PDFTRACT_H
#define PDFTRACT_H

#pragma once

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

/**
 * Get the ABI version of the library.
 *
 * # Returns
 *
 * A 32-bit unsigned integer encoding the ABI version.
 * Format: MAJOR << 16 | MINOR << 8 | PATCH
 *
 * For version 0.1.0, this returns 0x00000100 (256 decimal).
 * For version 1.2.3, this would return 0x010203 (66051 decimal).
 *
 * C callers can use this to verify the loaded library matches their
 * compiled header's expectations.
 */
uint32_t pdftract_abi_version(void);

/**
 * Classify a PDF file by type.
 *
 * # Arguments
 *
 * * `source` - Path to the PDF file (null-terminated UTF-8 string)
 *
 * # Returns
 *
 * A JSON string containing classification information. The caller MUST free this
 * with pdftract_free().
 *
 * # Note
 *
 * This is currently a stub that returns a basic classification.
 * Full implementation requires a trained classifier.
 */
char *pdftract_classify(const char *source);

/**
 * Extract text and structure from a PDF file.
 *
 * # Arguments
 *
 * * `source` - Path to the PDF file (null-terminated UTF-8 string)
 * * `options_json` - JSON string with extraction options (can be empty object "{}")
 *
 * # Returns
 *
 * A JSON string representing the extraction result. The caller MUST free this
 * with pdftract_free(). On error, returns a JSON object with "error" and "message" fields.
 *
 * # Example
 *
 * ```c
 * char *result = pdftract_extract("document.pdf", "{}");
 * // ... use result ...
 * pdftract_free(result);
 * ```
 */
char *pdftract_extract(const char *source,
                       const char *options_json);

/**
 * Extract markdown from a PDF file.
 *
 * # Arguments
 *
 * * `source` - Path to the PDF file (null-terminated UTF-8 string)
 * * `options_json` - JSON string with extraction options (can be empty object "{}")
 *
 * # Returns
 *
 * A JSON string containing the extracted markdown. The caller MUST free this
 * with pdftract_free().
 */
char *pdftract_extract_markdown(const char *source,
                                const char *options_json);

/**
 * Open a streaming extraction session.
 *
 * Returns an opaque handle that can be used with pdftract_stream_next()
 * to iterate through pages one at a time. When done, call pdftract_stream_close().
 *
 * # Arguments
 *
 * * `source` - Path to the PDF file (null-terminated UTF-8 string)
 * * `options_json` - JSON string with extraction options (can be empty object "{}")
 *
 * # Returns
 *
 * An opaque handle (*mut c_void) on success, or NULL on error.
 * Check for errors by examining the handle.
 */
void *pdftract_extract_stream_open(const char *source,
                                   const char *options_json);

/**
 * Extract plain text from a PDF file.
 *
 * # Arguments
 *
 * * `source` - Path to the PDF file (null-terminated UTF-8 string)
 * * `options_json` - JSON string with extraction options (can be empty object "{}")
 *
 * # Returns
 *
 * A JSON string containing the extracted text. The caller MUST free this
 * with pdftract_free().
 */
char *pdftract_extract_text(const char *source,
                            const char *options_json);

/**
 * Free a string returned by pdftract_* functions.
 *
 * # Arguments
 *
 * * `ptr` - Pointer to string returned by any pdftract_* function (except pdftract_version)
 *
 * # Safety
 *
 * This function MUST be called to free strings returned by the API.
 * Do NOT call libc free() on these pointers.
 */
void pdftract_free(char *ptr);

/**
 * Get metadata about a PDF file.
 *
 * # Arguments
 *
 * * `source` - Path to the PDF file (null-terminated UTF-8 string)
 * * `options_json` - JSON string with extraction options (can be empty object "{}")
 *
 * # Returns
 *
 * A JSON string containing PDF metadata. The caller MUST free this
 * with pdftract_free().
 */
char *pdftract_get_metadata(const char *source,
                            const char *options_json);

/**
 * Compute the cryptographic fingerprint of a PDF file.
 *
 * # Arguments
 *
 * * `source` - Path to the PDF file (null-terminated UTF-8 string)
 *
 * # Returns
 *
 * A JSON string containing the fingerprint. The caller MUST free this
 * with pdftract_free().
 */
char *pdftract_hash(const char *source);

/**
 * Get the last error message for the current thread.
 *
 * # Returns
 *
 * A pointer to a null-terminated string containing the last error message,
 * or NULL if no error has been set. The caller MUST NOT free this string.
 * The string remains valid until the next API call on this thread.
 *
 * # Note
 *
 * This function returns a pointer to thread-local storage that is invalidated
 * by the next API call on the same thread. If you need to retain the error
 * message, make a copy of it immediately.
 */
const char *pdftract_last_error(void);

/**
 * Search for text patterns in a PDF file.
 *
 * # Arguments
 *
 * * `source` - Path to the PDF file (null-terminated UTF-8 string)
 * * `pattern` - Search pattern (null-terminated UTF-8 string)
 * * `options_json` - JSON string with extraction options (can be empty object "{}")
 *
 * # Returns
 *
 * A JSON string containing search results. The caller MUST free this
 * with pdftract_free().
 */
char *pdftract_search(const char *source,
                      const char *pattern,
                      const char *options_json);

/**
 * Close a streaming extraction session and free resources.
 *
 * # Arguments
 *
 * * `handle` - Opaque handle from pdftract_extract_stream_open()
 */
void pdftract_stream_close(void *handle);

/**
 * Get the next page from a streaming extraction session.
 *
 * # Arguments
 *
 * * `handle` - Opaque handle from pdftract_extract_stream_open()
 *
 * # Returns
 *
 * A JSON string representing one page, or NULL when the stream ends.
 * The caller MUST free non-NULL returns with pdftract_free().
 *
 * # Note
 *
 * The handle remains valid after this call and must be closed with
 * pdftract_stream_close() when done.
 */
char *pdftract_stream_next(void *handle);

/**
 * Verify a visual citation receipt against a PDF file.
 *
 * # Arguments
 *
 * * `path` - Path to the PDF file (null-terminated UTF-8 string)
 * * `receipt_json` - JSON string containing the receipt to verify
 *
 * # Returns
 *
 * An int32_t exit code:
 * - 0: receipt verifies successfully
 * - 1: extraction failed (PDF unreadable, encrypted, etc.)
 * - 10: pdf_fingerprint mismatch
 * - 11: bbox mismatch (no span meets 90% IoU threshold)
 * - 12: content_hash mismatch (best-IoU span's text differs)
 *
 * On error, use pdftract_last_error() to get a detailed message.
 */
int32_t pdftract_verify_receipt(const char *path,
                                const char *receipt_json);

/**
 * Get the pdftract library version string.
 *
 * # Returns
 *
 * A static C string containing the version. Do NOT free this string.
 */
const char *pdftract_version(void);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif  /* PDFTRACT_H */
