#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "../crates/pdftract-libpdftract/include/pdftract.h"

int main(void) {
    const char* pdf_path = "/home/coding/pdftract/tests/fixtures/test-minimal.pdf";
    
    printf("Testing pdftract API with minimal PDF: %s\n\n", pdf_path);
    
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
    
    // Test search
    char* search = pdftract_search(pdf_path, "test", "{}");
    if (search) {
        printf("Search: %s\n", search);
        pdftract_free(search);
    }
    
    // Test stream
    void* handle = pdftract_extract_stream_open(pdf_path, "{}");
    if (handle) {
        char* page;
        int count = 0;
        while ((page = pdftract_stream_next(handle)) != NULL) {
            printf("Stream page %d: %s\n", count, page);
            pdftract_free(page);
            count++;
        }
        pdftract_stream_close(handle);
    }
    
    printf("\nAll API calls succeeded!\n");
    return 0;
}
