#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>
#include "../crates/pdftract-libpdftract/include/pdftract.h"

/* Use /tmp for the test PDF to avoid conflicts */
static const char* test_pdf_path = "/tmp/test-conformance.pdf";

static void create_test_pdf(const char* path) {
    FILE* f = fopen(path, "wb");
    assert(f != NULL);

    const char* pdf_content =
        "%PDF-1.4\n"
        "1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n"
        "2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj\n"
        "3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]"
        "/Resources<</Font<</F1 4 0 R>>>>/Contents 5 0 R>>endobj\n"
        "4 0 obj<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>endobj\n"
        "5 0 obj<</Length 44>>stream\n"
        "BT\n"
        "/F1 12 Tf\n"
        "50 700 Td\n"
        "(Hello World) Tj\n"
        "ET\n"
        "endstream\n"
        "endobj\n"
        "xref\n"
        "0 6\n"
        "0000000000 65535 f\n"
        "0000000009 00000 n\n"
        "0000000058 00000 n\n"
        "0000000115 00000 n\n"
        "0000000262 00000 n\n"
        "0000000331 00000 n\n"
        "trailer<</Size 6/Root 1 0 R>>\n"
        "startxref\n"
        "430\n"
        "%%EOF\n";

    fwrite(pdf_content, 1, strlen(pdf_content), f);
    fclose(f);
}

static void test_version(void) {
    const char* version = pdftract_version();
    assert(version != NULL);
    printf("[PASS] pdftract_version: %s\n", version);
}

static void test_abi_version(void) {
    uint32_t abi = pdftract_abi_version();
    printf("[PASS] pdftract_abi_version: 0x%08x\n", abi);
}

static void test_extract(void) {
    char* result = pdftract_extract(test_pdf_path, "{}");
    assert(result != NULL);
    printf("[PASS] pdftract_extract (%zu bytes)\n", strlen(result));
    pdftract_free(result);
}

static void test_extract_text(void) {
    char* result = pdftract_extract_text(test_pdf_path, "{}");
    assert(result != NULL);
    printf("[PASS] pdftract_extract_text (%zu bytes)\n", strlen(result));
    pdftract_free(result);
}

static void test_hash(void) {
    char* result = pdftract_hash(test_pdf_path);
    assert(result != NULL);
    printf("[PASS] pdftract_hash\n");
    pdftract_free(result);
}

static void test_null_pointers(void) {
    char* result = pdftract_extract(NULL, "{}");
    assert(result != NULL);
    printf("[PASS] null pointer handling\n");
    pdftract_free(result);
}

int main(void) {
    printf("=== libpdftract C Conformance Test ===\n\n");
    
    create_test_pdf(test_pdf_path);
    
    test_version();
    test_abi_version();
    test_hash();
    test_extract();
    test_extract_text();
    test_null_pointers();
    
    printf("\n=== All tests completed ===\n");
    remove(test_pdf_path);
    return 0;
}
