#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "../crates/pdftract-libpdftract/include/pdftract.h"

int main(void) {
    const char* pdf_path = "/home/coding/pdftract/tests/fixtures/classifier/contract/01.pdf";
    
    printf("Testing pdftract API with real PDF: %s\n\n", pdf_path);
    
    // Test hash
    char* hash = pdftract_hash(pdf_path);
    if (hash) {
        printf("Hash: %s\n", hash);
        pdftract_free(hash);
    }
    
    // Test extract_text
    char* text = pdftract_extract_text(pdf_path, "{}");
    if (text) {
        printf("Text: %s\n", text);
        pdftract_free(text);
    }
    
    // Test metadata
    char* meta = pdftract_get_metadata(pdf_path, "{}");
    if (meta) {
        printf("Metadata: %s\n", meta);
        pdftract_free(meta);
    }
    
    // Test classify
    char* classify = pdftract_classify(pdf_path);
    if (classify) {
        printf("Classify: %s\n", classify);
        pdftract_free(classify);
    }
    
    printf("\nAll API calls succeeded!\n");
    return 0;
}
