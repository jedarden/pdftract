/* Copyright 2026 Jed Cabanino. MIT OR Apache-2.0 */
/* Simple test for libpdftract C FFI API linking */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "../include/pdftract.h"

int main(void) {
    int failures = 0;

    /* Test 1: pdftract_version returns a valid string */
    {
        const char *version = pdftract_version();
        if (version == NULL || strlen(version) == 0) {
            fprintf(stderr, "FAIL: pdftract_version returned NULL or empty\n");
            failures++;
        } else {
            printf("PASS: pdftract_version() = %s\n", version);
        }
    }

    /* Test 2: pdftract_abi_version returns a non-zero value */
    {
        uint32_t abi = pdftract_abi_version();
        if (abi == 0) {
            fprintf(stderr, "FAIL: pdftract_abi_version returned 0\n");
            failures++;
        } else {
            printf("PASS: pdftract_abi_version() = 0x%08x\n", abi);
        }
    }

    /* Test 3: pdftract_free(NULL) is safe */
    {
        pdftract_free(NULL);
        printf("PASS: pdftract_free(NULL) is safe\n");
    }

    /* Test 4: pdftract_free works on allocated strings */
    {
        char *result = pdftract_hash("/dev/null");
        if (result != NULL) {
            /* Even if it's an error, it should be a valid string we can free */
            size_t len = strlen(result);
            printf("PASS: pdftract_hash returned string of length %zu\n", len);
            pdftract_free(result);
        } else {
            /* NULL is also acceptable for error cases */
            printf("PASS: pdftract_hash returned NULL (acceptable for error)\n");
        }
    }

    /* Test 5: All 9 contract methods are callable */
    {
        /* These may return NULL (errors), but the symbols should exist */
        char *r1 = pdftract_extract("/nonexistent.pdf", "{}");
        if (r1) pdftract_free(r1);
        printf("PASS: pdftract_extract is callable\n");

        char *r2 = pdftract_extract_text("/nonexistent.pdf", "{}");
        if (r2) pdftract_free(r2);
        printf("PASS: pdftract_extract_text is callable\n");

        char *r3 = pdftract_extract_markdown("/nonexistent.pdf", "{}");
        if (r3) pdftract_free(r3);
        printf("PASS: pdftract_extract_markdown is callable\n");

        void *handle = pdftract_extract_stream_open("/nonexistent.pdf", "{}");
        if (handle) pdftract_stream_close(handle);
        printf("PASS: pdftract_extract_stream_open is callable\n");

        char *r4 = pdftract_search("/nonexistent.pdf", "test", "{}");
        if (r4) pdftract_free(r4);
        printf("PASS: pdftract_search is callable\n");

        char *r5 = pdftract_get_metadata("/nonexistent.pdf", "{}");
        if (r5) pdftract_free(r5);
        printf("PASS: pdftract_get_metadata is callable\n");

        char *r6 = pdftract_hash("/nonexistent.pdf");
        if (r6) pdftract_free(r6);
        printf("PASS: pdftract_hash is callable\n");

        char *r7 = pdftract_classify("/nonexistent.pdf");
        if (r7) pdftract_free(r7);
        printf("PASS: pdftract_classify is callable\n");

        int32_t r8 = pdftract_verify_receipt("/nonexistent.pdf", "{}");
        (void)r8; /* suppress unused warning */
        printf("PASS: pdftract_verify_receipt is callable\n");
    }

    printf("\n=== Test Summary ===\n");
    if (failures == 0) {
        printf("All tests passed!\n");
        return 0;
    } else {
        printf("%d test(s) failed\n", failures);
        return 1;
    }
}
