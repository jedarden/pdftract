/* Copyright 2026 Jed Cabanino. MIT OR Apache-2.0 */

/*
 * Sample C client for pdftract library.
 * Tests basic extraction, null handling, and memory management.
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "../../crates/pdftract-libpdftract/include/pdftract.h"

/* Create a minimal test PDF */
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
    fwrite(pdf_data, 1, strlen(pdf_data), f);
    fclose(f);
    return 0;
}

/* Test 1: Basic extraction */
static int test_extract(const char *pdf_path) {
    printf("Test 1: Basic extraction... ");
    fflush(stdout);

    char *result = pdftract_extract(pdf_path, "{}");
    if (!result) {
        printf("FAILED (null result)\n");
        return 1;
    }

    /* Check that result looks like JSON */
    if (result[0] != '{') {
        printf("FAILED (not JSON)\n");
        pdftract_free(result);
        return 1;
    }

    printf("OK\n");
    pdftract_free(result);
    return 0;
}

/* Test 2: Null source handling */
static int test_null_source(void) {
    printf("Test 2: Null source handling... ");
    fflush(stdout);

    char *result = pdftract_extract(NULL, "{}");
    if (!result) {
        printf("FAILED (null result)\n");
        return 1;
    }

    /* Should be an error JSON */
    if (!strstr(result, "\"error\"")) {
        printf("FAILED (no error field)\n");
        pdftract_free(result);
        return 1;
    }

    printf("OK\n");
    pdftract_free(result);
    return 0;
}

/* Test 3: Null options handling */
static int test_null_options(const char *pdf_path) {
    printf("Test 3: Null options handling... ");
    fflush(stdout);

    char *result = pdftract_extract(pdf_path, NULL);
    if (!result) {
        printf("FAILED (null result)\n");
        return 1;
    }

    /* Should be an error JSON */
    if (!strstr(result, "\"error\"")) {
        printf("FAILED (no error field)\n");
        pdftract_free(result);
        return 1;
    }

    printf("OK\n");
    pdftract_free(result);
    return 0;
}

/* Test 4: Hash function */
static int test_hash(const char *pdf_path) {
    printf("Test 4: Hash function... ");
    fflush(stdout);

    char *result = pdftract_hash(pdf_path);
    if (!result) {
        printf("FAILED (null result)\n");
        return 1;
    }

    /* Check that result contains fingerprint */
    if (!strstr(result, "\"fingerprint\"")) {
        printf("FAILED (no fingerprint field)\n");
        pdftract_free(result);
        return 1;
    }

    printf("OK\n");
    pdftract_free(result);
    return 0;
}

/* Test 5: Metadata function */
static int test_metadata(const char *pdf_path) {
    printf("Test 5: Metadata function... ");
    fflush(stdout);

    char *result = pdftract_get_metadata(pdf_path, "{}");
    if (!result) {
        printf("FAILED (null result)\n");
        return 1;
    }

    /* Check that result has expected fields */
    if (!strstr(result, "\"page_count\"")) {
        printf("FAILED (no page_count field)\n");
        pdftract_free(result);
        return 1;
    }

    printf("OK\n");
    pdftract_free(result);
    return 0;
}

/* Test 6: Streaming API */
static int test_streaming(const char *pdf_path) {
    printf("Test 6: Streaming API... ");
    fflush(stdout);

    void *handle = pdftract_extract_stream_open(pdf_path, "{}");
    if (!handle) {
        printf("FAILED (null handle)\n");
        return 1;
    }

    /* Get first page */
    char *page = pdftract_stream_next(handle);
    if (!page) {
        printf("FAILED (null page)\n");
        pdftract_stream_close(handle);
        return 1;
    }

    /* Page should be JSON */
    if (page[0] != '{') {
        printf("FAILED (page not JSON)\n");
        pdftract_free(page);
        pdftract_stream_close(handle);
        return 1;
    }

    pdftract_free(page);

    /* Next call should return null (end of stream) */
    page = pdftract_stream_next(handle);
    if (page) {
        printf("FAILED (expected null at end)\n");
        pdftract_free(page);
        pdftract_stream_close(handle);
        return 1;
    }

    pdftract_stream_close(handle);
    printf("OK\n");
    return 0;
}

/* Test 7: Version function */
static int test_version(void) {
    printf("Test 7: Version function... ");
    fflush(stdout);

    const char *version = pdftract_version();
    if (!version) {
        printf("FAILED (null version)\n");
        return 1;
    }

    printf("OK (%s)\n", version);
    return 0;
}

/* Test 8: Memory roundtrip (leak check) */
static int test_memory_roundtrip(const char *pdf_path) {
    printf("Test 8: Memory roundtrip (1000 iterations)... ");
    fflush(stdout);

    for (int i = 0; i < 1000; i++) {
        char *result = pdftract_hash(pdf_path);
        if (!result) {
            printf("FAILED (null result at iteration %d)\n", i);
            return 1;
        }
        pdftract_free(result);
    }

    printf("OK\n");
    return 0;
}

/* Test 9: Search function */
static int test_search(const char *pdf_path) {
    printf("Test 9: Search function... ");
    fflush(stdout);

    char *result = pdftract_search(pdf_path, "test", "{}");
    if (!result) {
        printf("FAILED (null result)\n");
        return 1;
    }

    /* Check that result has expected fields */
    if (!strstr(result, "\"pattern\"")) {
        printf("FAILED (no pattern field)\n");
        pdftract_free(result);
        return 1;
    }

    printf("OK\n");
    pdftract_free(result);
    return 0;
}

/* Test 10: Classify function */
static int test_classify(const char *pdf_path) {
    printf("Test 10: Classify function... ");
    fflush(stdout);

    char *result = pdftract_classify(pdf_path);
    if (!result) {
        printf("FAILED (null result)\n");
        return 1;
    }

    /* Check that result has expected fields */
    if (!strstr(result, "\"type\"")) {
        printf("FAILED (no type field)\n");
        pdftract_free(result);
        return 1;
    }

    printf("OK\n");
    pdftract_free(result);
    return 0;
}

/* Test 11: Extract text function */
static int test_extract_text(const char *pdf_path) {
    printf("Test 11: Extract text function... ");
    fflush(stdout);

    char *result = pdftract_extract_text(pdf_path, "{}");
    if (!result) {
        printf("FAILED (null result)\n");
        return 1;
    }

    /* Result should be JSON */
    if (result[0] != '"' && result[0] != '{') {
        printf("FAILED (not JSON)\n");
        pdftract_free(result);
        return 1;
    }

    printf("OK\n");
    pdftract_free(result);
    return 0;
}

/* Test 12: Extract markdown function */
static int test_extract_markdown(const char *pdf_path) {
    printf("Test 12: Extract markdown function... ");
    fflush(stdout);

    char *result = pdftract_extract_markdown(pdf_path, "{}");
    if (!result) {
        printf("FAILED (null result)\n");
        return 1;
    }

    /* Result should be JSON */
    if (result[0] != '"' && result[0] != '{') {
        printf("FAILED (not JSON)\n");
        pdftract_free(result);
        return 1;
    }

    printf("OK\n");
    pdftract_free(result);
    return 0;
}

int main(void) {
    const char *test_pdf = "/tmp/test_pdftract.pdf";
    int failed = 0;

    printf("pdftract C client test\n");
    printf("=======================\n\n");

    /* Create test PDF */
    if (create_test_pdf(test_pdf) != 0) {
        fprintf(stderr, "Failed to create test PDF\n");
        return 1;
    }

    /* Run tests */
    failed += test_extract(test_pdf);
    failed += test_null_source();
    failed += test_null_options(test_pdf);
    failed += test_hash(test_pdf);
    failed += test_metadata(test_pdf);
    failed += test_streaming(test_pdf);
    failed += test_version();
    failed += test_memory_roundtrip(test_pdf);
    failed += test_search(test_pdf);
    failed += test_classify(test_pdf);
    failed += test_extract_text(test_pdf);
    failed += test_extract_markdown(test_pdf);

    /* Cleanup */
    remove(test_pdf);

    printf("\n");
    if (failed == 0) {
        printf("All tests passed!\n");
        return 0;
    } else {
        printf("%d test(s) failed\n", failed);
        return 1;
    }
}
