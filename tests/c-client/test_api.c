/* Copyright 2026 Jed Cabanino. MIT OR Apache-2.0 */

/**
 * C client test for pdftract FFI API.
 *
 * Tests the 12 exported functions:
 * - pdftract_extract
 * - pdftract_extract_text
 * - pdftract_extract_markdown
 * - pdftract_extract_stream_open
 * - pdftract_stream_next
 * - pdftract_stream_close
 * - pdftract_search
 * - pdftract_get_metadata
 * - pdftract_hash
 * - pdftract_classify
 * - pdftract_free
 * - pdftract_version
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>

// Include the generated header
#include "pdftract.h"

// Test PDF path - use a minimal PDF we'll create
#define TEST_PDF "../fixtures/minimal.pdf"

/**
 * Create a minimal valid PDF for testing.
 */
static int create_test_pdf(const char *path) {
    const char *pdf_data =
        "%PDF-1.4\n"
        "1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n"
        "2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj\n"
        "3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]/Resources<</Font<</F1<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>>>>>>>>>endobj\n"
        "xref\n"
        "0 4\n"
        "0000000000 65535 f\n"
        "0000000009 00000 n\n"
        "0000000052 00000 n\n"
        "0000000109 00000 n\n"
        "trailer<</Size 4/Root 1 0 R>>\n"
        "startxref\n"
        "206\n"
        "%%EOF\n";

    FILE *f = fopen(path, "w");
    if (!f) {
        perror("fopen");
        return 1;
    }
    size_t len = strlen(pdf_data);
    if (fwrite(pdf_data, 1, len, f) != len) {
        perror("fwrite");
        fclose(f);
        return 1;
    }
    fclose(f);
    return 0;
}

/**
 * Simple JSON parser to extract string values.
 * Returns a newly allocated string that must be freed by caller.
 */
static char *json_extract_string(const char *json, const char *key) {
    char search[256];
    snprintf(search, sizeof(search), "\"%s\"", key);

    const char *key_pos = strstr(json, search);
    if (!key_pos) {
        return NULL;
    }

    // Find the colon after the key
    const char *colon = strchr(key_pos, ':');
    if (!colon) {
        return NULL;
    }

    // Skip whitespace after colon
    const char *value_start = colon + 1;
    while (*value_start == ' ' || *value_start == '\t' || *value_start == '\n') {
        value_start++;
    }

    // Check if value is a string
    if (*value_start != '"') {
        return NULL;
    }
    value_start++;

    // Find the closing quote
    const char *value_end = strchr(value_start, '"');
    if (!value_end) {
        return NULL;
    }

    // Allocate and copy the string value
    size_t len = value_end - value_start;
    char *result = malloc(len + 1);
    if (result) {
        memcpy(result, value_start, len);
        result[len] = '\0';
    }
    return result;
}

/**
 * Check if JSON contains an error.
 */
static int json_has_error(const char *json) {
    return strstr(json, "\"error\"") != NULL;
}

/**
 * Extract error message from JSON.
 */
static char *json_extract_error(const char *json) {
    return json_extract_string(json, "message");
}

/**
 * Test pdftract_version.
 */
static void test_version(void) {
    printf("Testing pdftract_version...\n");
    const char *version = pdftract_version();
    assert(version != NULL);
    printf("  Version: %s\n", version);
    // Version should not be freed (static string)
    printf("  PASS\n\n");
}

/**
 * Test pdftract_hash.
 */
static void test_hash(const char *pdf_path) {
    printf("Testing pdftract_hash...\n");
    char *result = pdftract_hash(pdf_path);
    assert(result != NULL);

    if (json_has_error(result)) {
        char *err = json_extract_error(result);
        printf("  ERROR: %s\n", err ? err : result);
        free(err);
        pdftract_free(result);
        assert(0);
    }

    char *fingerprint = json_extract_string(result, "fingerprint");
    if (fingerprint) {
        printf("  Fingerprint: %s\n", fingerprint);
        free(fingerprint);
    }
    pdftract_free(result);
    printf("  PASS\n\n");
}

/**
 * Test pdftract_classify.
 */
static void test_classify(const char *pdf_path) {
    printf("Testing pdftract_classify...\n");
    char *result = pdftract_classify(pdf_path);
    assert(result != NULL);

    if (json_has_error(result)) {
        char *err = json_extract_error(result);
        printf("  ERROR: %s\n", err ? err : result);
        free(err);
        pdftract_free(result);
        assert(0);
    }

    printf("  Result: %s\n", result);
    pdftract_free(result);
    printf("  PASS\n\n");
}

/**
 * Test pdftract_get_metadata.
 */
static void test_get_metadata(const char *pdf_path) {
    printf("Testing pdftract_get_metadata...\n");
    char *result = pdftract_get_metadata(pdf_path, "{}");
    assert(result != NULL);

    if (json_has_error(result)) {
        char *err = json_extract_error(result);
        printf("  ERROR: %s\n", err ? err : result);
        free(err);
        pdftract_free(result);
        assert(0);
    }

    printf("  Metadata: %s\n", result);
    pdftract_free(result);
    printf("  PASS\n\n");
}

/**
 * Test pdftract_extract.
 */
static void test_extract(const char *pdf_path) {
    printf("Testing pdftract_extract...\n");
    char *result = pdftract_extract(pdf_path, "{}");
    assert(result != NULL);

    if (json_has_error(result)) {
        char *err = json_extract_error(result);
        printf("  ERROR: %s\n", err ? err : result);
        free(err);
        pdftract_free(result);
        assert(0);
    }

    printf("  Extracted (first 100 chars): %.100s%s\n",
           result, strlen(result) > 100 ? "..." : "");
    pdftract_free(result);
    printf("  PASS\n\n");
}

/**
 * Test pdftract_extract_text.
 */
static void test_extract_text(const char *pdf_path) {
    printf("Testing pdftract_extract_text...\n");
    char *result = pdftract_extract_text(pdf_path, "{}");
    assert(result != NULL);

    if (json_has_error(result)) {
        char *err = json_extract_error(result);
        printf("  ERROR: %s\n", err ? err : result);
        free(err);
        pdftract_free(result);
        assert(0);
    }

    printf("  Text: %s\n", result);
    pdftract_free(result);
    printf("  PASS\n\n");
}

/**
 * Test pdftract_extract_markdown.
 */
static void test_extract_markdown(const char *pdf_path) {
    printf("Testing pdftract_extract_markdown...\n");
    char *result = pdftract_extract_markdown(pdf_path, "{}");
    assert(result != NULL);

    if (json_has_error(result)) {
        char *err = json_extract_error(result);
        printf("  ERROR: %s\n", err ? err : result);
        free(err);
        pdftract_free(result);
        assert(0);
    }

    printf("  Markdown: %s\n", result);
    pdftract_free(result);
    printf("  PASS\n\n");
}

/**
 * Test streaming API.
 */
static void test_stream(const char *pdf_path) {
    printf("Testing streaming API...\n");
    void *handle = pdftract_extract_stream_open(pdf_path, "{}");
    assert(handle != NULL);

    int page_count = 0;
    char *page;
    while ((page = pdftract_stream_next(handle)) != NULL) {
        page_count++;
        printf("  Page %d: %.50s...\n", page_count, page);
        pdftract_free(page);
    }

    pdftract_stream_close(handle);
    printf("  Total pages: %d\n", page_count);
    printf("  PASS\n\n");
}

/**
 * Test pdftract_search.
 */
static void test_search(const char *pdf_path) {
    printf("Testing pdftract_search...\n");
    char *result = pdftract_search(pdf_path, "test", "{}");
    assert(result != NULL);

    if (json_has_error(result)) {
        char *err = json_extract_error(result);
        printf("  ERROR: %s\n", err ? err : result);
        free(err);
        pdftract_free(result);
        assert(0);
    }

    printf("  Search result: %s\n", result);
    pdftract_free(result);
    printf("  PASS\n\n");
}

/**
 * Test null pointer handling.
 */
static void test_null_pointers(void) {
    printf("Testing null pointer handling...\n");

    // Null source should return error JSON, not crash
    char *result = pdftract_extract(NULL, "{}");
    assert(result != NULL);
    assert(json_has_error(result));
    pdftract_free(result);

    // Null options_json should return error JSON, not crash
    result = pdftract_extract(TEST_PDF, NULL);
    assert(result != NULL);
    assert(json_has_error(result));
    pdftract_free(result);

    // pdftract_free with null should not crash
    pdftract_free(NULL);
    pdftract_stream_close(NULL);

    printf("  PASS (no crashes on null pointers)\n\n");
}

/**
 * Test pdftract_free roundtrip.
 */
static void test_free_roundtrip(void) {
    printf("Testing pdftract_free roundtrip...\n");

    // Allocate and free many times to ensure no leaks
    for (int i = 0; i < 100; i++) {
        char *result = pdftract_version();
        // Version is static, don't free it
        (void)result;

        result = pdftract_hash(TEST_PDF);
        if (result && !json_has_error(result)) {
            pdftract_free(result);
        }
    }

    printf("  PASS (100 alloc/free cycles completed)\n\n");
}

int main(void) {
    printf("=== pdftract C Client Test ===\n\n");

    // Create test PDF
    if (create_test_pdf(TEST_PDF) != 0) {
        fprintf(stderr, "Failed to create test PDF\n");
        return 1;
    }

    // Run all tests
    test_version();
    test_hash(TEST_PDF);
    test_classify(TEST_PDF);
    test_get_metadata(TEST_PDF);
    test_extract(TEST_PDF);
    test_extract_text(TEST_PDF);
    test_extract_markdown(TEST_PDF);
    test_stream(TEST_PDF);
    test_search(TEST_PDF);
    test_null_pointers();
    test_free_roundtrip();

    printf("=== All tests passed! ===\n");

    // Clean up
    remove(TEST_PDF);

    return 0;
}
