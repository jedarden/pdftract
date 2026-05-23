#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>
#include "pdftract.h"

#define TEST_PDF "fixtures/minimal.pdf"

static int json_has_error(const char *json) {
    return strstr(json, "\"error\"") != NULL;
}

int main(void) {
    printf("=== pdftract C Client Test ===\n\n");

    // Test version
    printf("Testing pdftract_version...\n");
    const char *version = pdftract_version();
    printf("  Version: %s\n", version);
    printf("  PASS\n\n");

    // Test hash
    printf("Testing pdftract_hash...\n");
    char *result = pdftract_hash(TEST_PDF);
    if (json_has_error(result)) {
        printf("  ERROR: %s\n", result);
        pdftract_free(result);
        return 1;
    }
    printf("  Hash: %.100s...\n", result);
    pdftract_free(result);
    printf("  PASS\n\n");

    // Test classify
    printf("Testing pdftract_classify...\n");
    result = pdftract_classify(TEST_PDF);
    if (json_has_error(result)) {
        printf("  ERROR: %s\n", result);
        pdftract_free(result);
        return 1;
    }
    printf("  Classify: %.100s...\n", result);
    pdftract_free(result);
    printf("  PASS\n\n");

    // Test extract
    printf("Testing pdftract_extract...\n");
    result = pdftract_extract(TEST_PDF, "{}");
    if (json_has_error(result)) {
        printf("  ERROR: %s\n", result);
        pdftract_free(result);
        return 1;
    }
    printf("  Extract: %.200s...\n", result);
    pdftract_free(result);
    printf("  PASS\n\n");

    // Test extract_text
    printf("Testing pdftract_extract_text...\n");
    result = pdftract_extract_text(TEST_PDF, "{}");
    if (json_has_error(result)) {
        printf("  ERROR: %s\n", result);
        pdftract_free(result);
        return 1;
    }
    printf("  Text: %.100s...\n", result);
    pdftract_free(result);
    printf("  PASS\n\n");

    // Test extract_markdown
    printf("Testing pdftract_extract_markdown...\n");
    result = pdftract_extract_markdown(TEST_PDF, "{}");
    if (json_has_error(result)) {
        printf("  ERROR: %s\n", result);
        pdftract_free(result);
        return 1;
    }
    printf("  Markdown: %.100s...\n", result);
    pdftract_free(result);
    printf("  PASS\n\n");

    // Test stream
    printf("Testing streaming API...\n");
    void *handle = pdftract_extract_stream_open(TEST_PDF, "{}");
    if (!handle) {
        printf("  ERROR: failed to open stream\n");
        return 1;
    }
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

    // Test search
    printf("Testing pdftract_search...\n");
    result = pdftract_search(TEST_PDF, "Test", "{}");
    if (json_has_error(result)) {
        printf("  ERROR: %s\n", result);
        pdftract_free(result);
        return 1;
    }
    printf("  Search: %.100s...\n", result);
    pdftract_free(result);
    printf("  PASS\n\n");

    // Test get_metadata
    printf("Testing pdftract_get_metadata...\n");
    result = pdftract_get_metadata(TEST_PDF, "{}");
    if (json_has_error(result)) {
        printf("  ERROR: %s\n", result);
        pdftract_free(result);
        return 1;
    }
    printf("  Metadata: %.100s...\n", result);
    pdftract_free(result);
    printf("  PASS\n\n");

    // Test null handling
    printf("Testing null pointer handling...\n");
    result = pdftract_extract(NULL, "{}");
    assert(result != NULL);
    assert(json_has_error(result));
    pdftract_free(result);
    
    result = pdftract_extract(TEST_PDF, NULL);
    assert(result != NULL);
    assert(json_has_error(result));
    pdftract_free(result);
    
    pdftract_free(NULL);
    pdftract_stream_close(NULL);
    printf("  PASS\n\n");

    printf("=== All tests passed! ===\n");
    return 0;
}
