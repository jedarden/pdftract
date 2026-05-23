#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "pdftract.h"

void test_and_free(const char *name, char *result) {
    printf("%s: ", name);
    if (!result) {
        printf("FAIL - NULL result\n");
        return;
    }
    if (strstr(result, "\"error\"")) {
        printf("FAIL - %s\n", result);
    } else {
        printf("PASS\n");
        if (strlen(result) < 200) {
            printf("  Result: %s\n", result);
        } else {
            printf("  Result (truncated): %.150s...\n", result);
        }
    }
    pdftract_free(result);
}

int main(void) {
    printf("=== pdftract C API Conformance ===\n\n");
    
    const char *pdf_path = "/home/coding/pdftract/crates/pdftract-core/__test__.pdf";
    
    printf("Library: %s (ABI %u)\n\n", pdftract_version(), pdftract_abi_version());
    
    test_and_free("hash", pdftract_hash(pdf_path));
    test_and_free("classify", pdftract_classify(pdf_path));
    test_and_free("extract_text", pdftract_extract_text(pdf_path, "{}"));
    test_and_free("get_metadata", pdftract_get_metadata(pdf_path, "{}"));
    test_and_free("extract_markdown", pdftract_extract_markdown(pdf_path, "{}"));
    
    printf("\n=== Stream API Tests ===\n");
    
    void *stream = pdftract_extract_stream_open(pdf_path, "{}");
    if (stream) {
        printf("stream_open: PASS\n");
        char *page = pdftract_stream_next(stream);
        if (page) {
            printf("stream_next: PASS\n");
            pdftract_free(page);
        } else {
            printf("stream_next: FAIL - NULL page\n");
        }
        pdftract_stream_close(stream);
        printf("stream_close: PASS\n");
    } else {
        printf("stream_open: FAIL - NULL handle\n");
    }
    
    printf("\n=== Search & Verify Tests ===\n");
    
    test_and_free("search", pdftract_search(pdf_path, "test", "{}"));
    
    int32_t verify_result = pdftract_verify_receipt(pdf_path, "{}");
    printf("verify_receipt: %s (code=%d)\n", 
           verify_result == 1 ? "PASS (expected failure)" : "result", verify_result);
    
    printf("\n=== Test Complete ===\n");
    return 0;
}
