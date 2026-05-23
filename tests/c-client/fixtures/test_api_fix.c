#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>
#include "../../crates/pdftract-libpdftract/include/pdftract.h"

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

    // Test null handling
    printf("Testing null pointer handling...\n");
    result = pdftract_extract(NULL, "{}");
    assert(result != NULL);
    assert(json_has_error(result));
    pdftract_free(result);
    printf("  PASS\n\n");

    printf("=== All tests passed! ===\n");
    return 0;
}
