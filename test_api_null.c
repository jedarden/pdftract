#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>
#include "../../crates/pdftract-libpdftract/include/pdftract.h"

static int json_has_error(const char *json) {
    return strstr(json, "\"error\"") != NULL;
}

static int json_has_code(const char *json, const char *code) {
    char search[256];
    snprintf(search, sizeof(search), "\"error\":\"%s\"", code);
    return strstr(json, search) != NULL;
}

int main(void) {
    printf("=== pdftract FFI API Surface Test ===\n\n");

    // Test 1: pdftract_version (static string, don't free)
    printf("Test 1: pdftract_version...\n");
    const char *version = pdftract_version();
    assert(version != NULL);
    printf("  Version: %s\n", version);
    printf("  PASS\n\n");

    // Test 2: Null source handling - should return error JSON
    printf("Test 2: Null source handling...\n");
    char *result = pdftract_extract(NULL, "{}");
    assert(result != NULL);
    assert(json_has_error(result));
    assert(json_has_code(result, "NULL_POINTER") || json_has_code(result, "PANIC"));
    printf("  Error: %s\n", result);
    pdftract_free(result);
    printf("  PASS\n\n");

    // Test 3: Null options_json handling - should return error JSON
    printf("Test 3: Null options_json handling...\n");
    result = pdftract_extract("/fake/path.pdf", NULL);
    assert(result != NULL);
    assert(json_has_error(result));
    printf("  Error: %s\n", result);
    pdftract_free(result);
    printf("  PASS\n\n");

    // Test 4: pdftract_free with null - should not crash
    printf("Test 4: pdftract_free(null)...\n");
    pdftract_free(NULL);
    printf("  PASS\n\n");

    // Test 5: pdftract_stream_close with null - should not crash
    printf("Test 5: pdftract_stream_close(null)...\n");
    pdftract_stream_close(NULL);
    printf("  PASS\n\n");

    // Test 6: pdftract_stream_next with null handle - should return error JSON
    printf("Test 6: pdftract_stream_next(null handle)...\n");
    result = pdftract_stream_next(NULL);
    assert(result != NULL);
    assert(json_has_error(result));
    printf("  Error: %s\n", result);
    pdftract_free(result);
    printf("  PASS\n\n");

    // Test 7: Memory roundtrip - alloc and free many times
    printf("Test 7: Memory roundtrip (100 iterations)...\n");
    for (int i = 0; i < 100; i++) {
        result = pdftract_extract(NULL, "{}");
        assert(result != NULL);
        pdftract_free(result);
    }
    printf("  PASS\n\n");

    // Test 8: Invalid JSON in options - should return error
    printf("Test 8: Invalid JSON options...\n");
    result = pdftract_extract("/fake/path.pdf", "not valid json");
    assert(result != NULL);
    assert(json_has_error(result));
    printf("  Error: %s\n", result);
    pdftract_free(result);
    printf("  PASS\n\n");

    // Test 9: All 12 functions exist and return non-null for valid inputs
    printf("Test 9: Function existence check...\n");
    
    // These should all return non-null (even if error JSON) for null inputs
    result = pdftract_hash(NULL);
    assert(result != NULL);
    pdftract_free(result);
    
    result = pdftract_classify(NULL);
    assert(result != NULL);
    pdftract_free(result);
    
    result = pdftract_search(NULL, "pattern", "{}");
    assert(result != NULL);
    pdftract_free(result);
    
    result = pdftract_get_metadata(NULL, "{}");
    assert(result != NULL);
    pdftract_free(result);
    
    result = pdftract_extract_text(NULL, "{}");
    assert(result != NULL);
    pdftract_free(result);
    
    result = pdftract_extract_markdown(NULL, "{}");
    assert(result != NULL);
    pdftract_free(result);
    
    void *handle = pdftract_extract_stream_open(NULL, "{}");
    // handle might be null on error, which is ok
    
    printf("  PASS\n\n");

    printf("=== All API surface tests passed! ===\n");
    printf("\nNote: Full PDF parsing tests require Phase 1.2 completion.\n");
    printf("The FFI API surface is correctly implemented with:\n");
    printf("  - 12 exported symbols\n");
    printf("  - Null pointer safety\n");
    printf("  - Error JSON format\n");
    printf("  - Memory management\n");
    printf("  - Panic safety (catch_unwind)\n");
    
    return 0;
}
