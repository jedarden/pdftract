#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "pdftract.h"

int main(void) {
    printf("=== pdftract C API Test ===\n\n");
    
    printf("Version: %s\n", pdftract_version());
    printf("ABI Version: %u\n\n", pdftract_abi_version());
    
    const char *pdf_path = "/tmp/test_minimal.pdf";
    
    // Test hash
    printf("Testing pdftract_hash...\n");
    char *hash_result = pdftract_hash(pdf_path);
    if (hash_result) {
        printf("Result: %s\n", hash_result);
        if (!strstr(hash_result, "\"error\"")) {
            printf("PASS: hash succeeded\n");
        }
        pdftract_free(hash_result);
    }
    
    // Test extract_text
    printf("\nTesting pdftract_extract_text...\n");
    char *text_result = pdftract_extract_text(pdf_path, "{}");
    if (text_result) {
        if (strlen(text_result) > 10) {
            printf("Text (first 100 chars): %.100s...\n", text_result);
            printf("PASS: extract_text succeeded\n");
        } else {
            printf("Result: %s\n", text_result);
        }
        pdftract_free(text_result);
    }
    
    // Test classify
    printf("\nTesting pdftract_classify...\n");
    char *classify_result = pdftract_classify(pdf_path);
    if (classify_result) {
        printf("Result: %s\n", classify_result);
        if (!strstr(classify_result, "\"error\"")) {
            printf("PASS: classify succeeded\n");
        }
        pdftract_free(classify_result);
    }
    
    printf("\n=== All tests completed ===\n");
    return 0;
}
