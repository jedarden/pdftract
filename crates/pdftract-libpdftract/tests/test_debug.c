#include <stdio.h>
#include <string.h>
#include "../include/pdftract.h"

int main() {
    printf("=== Testing libpdftract ===\n\n");
    
    // Test version
    const char *version = pdftract_version();
    printf("Version: %s\n", version);
    
    // Test ABI version
    uint32_t abi = pdftract_abi_version();
    printf("ABI Version: 0x%08x\n", abi);
    
    // Test free NULL
    pdftract_free(NULL);
    printf("free(NULL): OK\n");
    
    // Test hash with nonexistent file
    printf("\nTesting nonexistent file:\n");
    char *result = pdftract_hash("/nonexistent/file.pdf");
    if (result == NULL) {
        printf("  Result: NULL\n");
        const char *err = pdftract_last_error();
        if (err) printf("  Error: %s\n", err);
    } else {
        printf("  Result: %s\n", result);
        pdftract_free(result);
    }
    
    // Test with valid PDF
    printf("\nTesting valid-minimal.pdf:\n");
    result = pdftract_hash("/home/coding/pdftract/tests/fixtures/valid-minimal.pdf");
    if (result == NULL) {
        printf("  Result: NULL\n");
        const char *err = pdftract_last_error();
        if (err) printf("  Error: %s\n", err);
    } else {
        printf("  Result: %s\n", result);
        if (strstr(result, "\"error\"") == NULL) {
            printf("  SUCCESS: Got valid response\n");
        } else {
            printf("  Got error response\n");
        }
        pdftract_free(result);
    }
    
    // Test extract_text
    printf("\nTesting extract_text:\n");
    result = pdftract_extract_text("/home/coding/pdftract/tests/fixtures/valid-minimal.pdf", "{}");
    if (result == NULL) {
        printf("  Result: NULL\n");
        const char *err = pdftract_last_error();
        if (err) printf("  Error: %s\n", err);
    } else {
        printf("  Result: %s\n", result);
        pdftract_free(result);
    }
    
    // Test classify
    printf("\nTesting classify:\n");
    result = pdftract_classify("/home/coding/pdftract/tests/fixtures/valid-minimal.pdf");
    if (result == NULL) {
        printf("  Result: NULL\n");
        const char *err = pdftract_last_error();
        if (err) printf("  Error: %s\n", err);
    } else {
        printf("  Result: %s\n", result);
        pdftract_free(result);
    }
    
    // Test get_metadata
    printf("\nTesting get_metadata:\n");
    result = pdftract_get_metadata("/home/coding/pdftract/tests/fixtures/valid-minimal.pdf", "{}");
    if (result == NULL) {
        printf("  Result: NULL\n");
        const char *err = pdftract_last_error();
        if (err) printf("  Error: %s\n", err);
    } else {
        printf("  Result: %s\n", result);
        pdftract_free(result);
    }
    
    return 0;
}
