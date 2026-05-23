#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>
#include "../crates/pdftract-libpdftract/include/pdftract.h"

static void create_test_pdf(const char* path) {
    FILE* f = fopen(path, "wb");
    assert(f != NULL);

    /* A more complete minimal PDF */
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

int main(void) {
    printf("=== Simple C Conformance Test ===\n\n");
    
    create_test_pdf("tests/fixtures/test-simple.pdf");
    
    /* Test basic functions */
    printf("Testing pdftract_version...\n");
    const char* version = pdftract_version();
    printf("  Version: %s\n", version);
    
    printf("Testing pdftract_abi_version...\n");
    uint32_t abi = pdftract_abi_version();
    printf("  ABI: 0x%08x\n", abi);
    
    /* Test extraction functions */
    printf("Testing pdftract_hash...\n");
    char* result = pdftract_hash("tests/fixtures/test-simple.pdf");
    printf("  Result: %s\n", result);
    if (result) pdftract_free(result);
    
    printf("Testing pdftract_extract_text...\n");
    char* text = pdftract_extract_text("tests/fixtures/test-simple.pdf", "{}");
    printf("  Result: %.100s%s\n", text, strlen(text) > 100 ? "..." : "");
    if (text) pdftract_free(text);
    
    printf("Testing pdftract_get_metadata...\n");
    char* meta = pdftract_get_metadata("tests/fixtures/test-simple.pdf", "{}");
    printf("  Result: %.100s%s\n", meta, strlen(meta) > 100 ? "..." : "");
    if (meta) pdftract_free(meta);
    
    printf("\n=== Tests completed ===\n");
    remove("tests/fixtures/test-simple.pdf");
    return 0;
}
