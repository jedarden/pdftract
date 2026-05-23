#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "../../crates/pdftract-libpdftract/include/pdftract.h"

int main(void) {
    const char *pdf_path = "/tmp/valid_test.pdf";

    // Test hash function
    printf("Testing hash function...\n");
    char *result = pdftract_hash(pdf_path);
    if (result) {
        printf("Hash result: %s\n", result);
        if (strstr(result, "\"fingerprint\"")) {
            printf("PASS: Hash contains fingerprint field\n");
        } else {
            printf("FAIL: Hash missing fingerprint field\n");
        }
        pdftract_free(result);
    } else {
        printf("Hash returned null\n");
    }

    // Test extract function
    printf("\nTesting extract function...\n");
    result = pdftract_extract(pdf_path, "{}");
    if (result) {
        printf("Extract result (first 500 chars): %.500s...\n", result);
        if (result[0] == '{') {
            printf("PASS: Extract returns JSON\n");
        }
        pdftract_free(result);
    } else {
        printf("Extract returned null\n");
    }

    // Test get_metadata
    printf("\nTesting get_metadata function...\n");
    result = pdftract_get_metadata(pdf_path, "{}");
    if (result) {
        printf("Metadata result: %s\n", result);
        if (strstr(result, "\"page_count\"")) {
            printf("PASS: Metadata contains page_count field\n");
        } else {
            printf("FAIL: Metadata missing page_count field\n");
        }
        pdftract_free(result);
    } else {
        printf("get_metadata returned null\n");
    }

    // Test streaming
    printf("\nTesting streaming API...\n");
    void *handle = pdftract_extract_stream_open(pdf_path, "{}");
    if (handle) {
        char *page = pdftract_stream_next(handle);
        if (page) {
            printf("Stream page (first 200 chars): %.200s...\n", page);
            pdftract_free(page);
            printf("PASS: Streaming works\n");
        } else {
            printf("FAIL: Stream returned null page\n");
        }
        page = pdftract_stream_next(handle);
        if (page == NULL) {
            printf("PASS: Stream correctly returns NULL at end\n");
        } else {
            printf("FAIL: Stream should return NULL at end, got: %s\n", page);
            pdftract_free(page);
        }
        pdftract_stream_close(handle);
    } else {
        printf("FAIL: Stream open returned null handle\n");
    }

    return 0;
}
