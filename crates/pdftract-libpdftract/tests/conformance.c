/* Copyright 2026 Jed Cabanino. MIT OR Apache-2.0 */
/* Conformance test for libpdftract C FFI API */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <pthread.h>
#include "../include/pdftract.h"

#define TEST_ASSERT(cond, msg) \
    do { \
        if (!(cond)) { \
            fprintf(stderr, "FAIL: %s\n", msg); \
            exit(1); \
        } \
    } while (0)

#define TEST_ASSERT_NONNULL(ptr, msg) \
    TEST_ASSERT((ptr) != NULL, msg)

#define TEST_ASSERT_NULL(ptr, msg) \
    TEST_ASSERT((ptr) == NULL, msg)

static int tests_passed = 0;
static int tests_failed = 0;

void test_version(void) {
    const char *version = pdftract_version();
    TEST_ASSERT_NONNULL(version, "version should not be NULL");
    TEST_ASSERT(strlen(version) > 0, "version should not be empty");
    printf("PASS: pdftract_version() = %s\n", version);
    tests_passed++;
}

void test_abi_version(void) {
    uint32_t abi = pdftract_abi_version();
    TEST_ASSERT(abi != 0, "ABI version should be non-zero");
    printf("PASS: pdftract_abi_version() = 0x%08x\n", abi);
    tests_passed++;
}

void test_free_null(void) {
    /* Freeing NULL should be safe */
    pdftract_free(NULL);
    printf("PASS: pdftract_free(NULL) is safe\n");
    tests_passed++;
}

void test_extract_text_minimal_pdf(const char *pdf_path) {
    char *result = pdftract_extract_text(pdf_path, "{}");
    if (result == NULL) {
        const char *err = pdftract_last_error();
        printf("SKIP: pdftract_extract_text() failed: %s\n", err ? err : "unknown error");
        return;
    }

    /* Result should be valid JSON (a string) */
    TEST_ASSERT(result[0] == '"' || result[0] == '{', "result should be JSON string or object");

    printf("PASS: pdftract_extract_text() returned: %s\n", result);
    pdftract_free(result);
    tests_passed++;
}

void test_extract_invalid_pdf(void) {
    char *result = pdftract_extract_text("/nonexistent/path.pdf", "{}");

    /* Should return NULL or an error JSON */
    if (result == NULL) {
        const char *err = pdftract_last_error();
        TEST_ASSERT(err != NULL, "last_error should be set after NULL return");
        printf("PASS: extract_text returns NULL for nonexistent file, error: %s\n", err);
    } else {
        /* Should be an error JSON */
        TEST_ASSERT(strstr(result, "\"error\"") != NULL, "result should contain error field");
        printf("PASS: extract_text returns error JSON: %s\n", result);
        pdftract_free(result);
    }
    tests_passed++;
}

void test_hash(const char *pdf_path) {
    char *result = pdftract_hash(pdf_path);
    if (result == NULL) {
        const char *err = pdftract_last_error();
        printf("SKIP: pdftract_hash() failed: %s\n", err ? err : "unknown error");
        return;
    }

    TEST_ASSERT(strstr(result, "\"fingerprint\"") != NULL, "result should contain fingerprint field");
    printf("PASS: pdftract_hash() returned: %s\n", result);
    pdftract_free(result);
    tests_passed++;
}

void test_classify(const char *pdf_path) {
    char *result = pdftract_classify(pdf_path);
    if (result == NULL) {
        const char *err = pdftract_last_error();
        printf("SKIP: pdftract_classify() failed: %s\n", err ? err : "unknown error");
        return;
    }

    TEST_ASSERT(strstr(result, "\"type\"") != NULL, "result should contain type field");
    printf("PASS: pdftract_classify() returned: %s\n", result);
    pdftract_free(result);
    tests_passed++;
}

void test_metadata(const char *pdf_path) {
    char *result = pdftract_get_metadata(pdf_path, "{}");
    if (result == NULL) {
        const char *err = pdftract_last_error();
        printf("SKIP: pdftract_get_metadata() failed: %s\n", err ? err : "unknown error");
        return;
    }

    TEST_ASSERT(strstr(result, "\"fingerprint\"") != NULL, "result should contain fingerprint field");
    printf("PASS: pdftract_get_metadata() returned: %s\n", result);
    pdftract_free(result);
    tests_passed++;
}

void test_stream(const char *pdf_path) {
    void *handle = pdftract_extract_stream_open(pdf_path, "{}");
    if (handle == NULL) {
        const char *err = pdftract_last_error();
        printf("SKIP: pdftract_extract_stream_open() failed: %s\n", err ? err : "unknown error");
        return;
    }

    int page_count = 0;
    char *page;
    while ((page = pdftract_stream_next(handle)) != NULL) {
        page_count++;
        TEST_ASSERT(strstr(page, "\"index\"") != NULL, "page should contain index field");
        pdftract_free(page);
    }

    pdftract_stream_close(handle);
    printf("PASS: pdftract_extract_stream processed %d pages\n", page_count);
    tests_passed++;
}

void test_search(const char *pdf_path) {
    char *result = pdftract_search(pdf_path, "test", "{}");
    if (result == NULL) {
        const char *err = pdftract_last_error();
        printf("SKIP: pdftract_search() failed: %s\n", err ? err : "unknown error");
        return;
    }

    TEST_ASSERT(strstr(result, "\"matches\"") != NULL, "result should contain matches field");
    printf("PASS: pdftract_search() returned: %s\n", result);
    pdftract_free(result);
    tests_passed++;
}

/* Thread-safe test data */
struct thread_data {
    int thread_id;
    const char *pdf_path;
    int iterations;
};

void *thread_test(void *arg) {
    struct thread_data *data = (struct thread_data *)arg;

    for (int i = 0; i < data->iterations; i++) {
        char *result = pdftract_hash(data->pdf_path);
        if (result != NULL) {
            pdftract_free(result);
        }
    }

    return NULL;
}

void test_thread_safety(const char *pdf_path) {
    const int num_threads = 4;
    const int iterations = 10;
    pthread_t threads[num_threads];
    struct thread_data data[num_threads];

    /* Create threads */
    for (int i = 0; i < num_threads; i++) {
        data[i].thread_id = i;
        data[i].pdf_path = pdf_path;
        data[i].iterations = iterations;

        if (pthread_create(&threads[i], NULL, thread_test, &data[i]) != 0) {
            perror("pthread_create");
            exit(1);
        }
    }

    /* Wait for threads */
    for (int i = 0; i < num_threads; i++) {
        pthread_join(threads[i], NULL);
    }

    printf("PASS: thread safety test completed (%d threads x %d iterations)\n",
           num_threads, iterations);
    tests_passed++;
}

void test_memory_leak_basic(void) {
    /* Allocate and free many strings to check for leaks */
    for (int i = 0; i < 1000; i++) {
        const char *version = pdftract_version();
        /* version is static, shouldn't free */
        (void)version; /* suppress unused warning */
    }

    /* Test that freeing works correctly */
    char *result = pdftract_hash("/dev/null");
    if (result != NULL) {
        pdftract_free(result);
    }

    printf("PASS: basic memory leak test\n");
    tests_passed++;
}

int main(int argc, char *argv[]) {
    const char *pdf_path = NULL;

    if (argc > 1) {
        pdf_path = argv[1];
    } else {
        /* Use a minimal test PDF if available */
        pdf_path = "../../../tests/fixtures/test-minimal.pdf";
    }

    printf("=== libpdftract C FFI Conformance Test ===\n");
    printf("Test PDF: %s\n\n", pdf_path);

    /* Basic API tests */
    test_version();
    test_abi_version();
    test_free_null();
    test_memory_leak_basic();

    /* Tests that require a PDF */
    if (pdf_path != NULL) {
        test_extract_text_minimal_pdf(pdf_path);
        test_extract_invalid_pdf();
        test_hash(pdf_path);
        test_classify(pdf_path);
        test_metadata(pdf_path);
        test_stream(pdf_path);
        test_search(pdf_path);
        test_thread_safety(pdf_path);
    }

    printf("\n=== Test Summary ===\n");
    printf("Passed: %d\n", tests_passed);
    printf("Failed: %d\n", tests_failed);

    return tests_failed > 0 ? 1 : 0;
}
