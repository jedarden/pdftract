#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "../crates/pdftract-libpdftract/include/pdftract.h"

int main(void) {
    const char* pdf_path = "/tmp/test_valid.pdf";
    
    printf("Testing pdftract API with valid PDF: %s\n\n", pdf_path);
    
    // Test version
    const char* version = pdftract_version();
    printf("Version: %s\n", version);
    
    // Test ABI version
    uint32_t abi = pdftract_abi_version();
    printf("ABI version: 0x%08x\n", abi);
    
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
    char* search = pdftract_search(pdf_path, "Hello", "{}");
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
    } else {
        printf("Stream open failed (handle is NULL)\n");
    }
    
    // Test markdown
    char* md = pdftract_extract_markdown(pdf_path, "{}");
    if (md) {
        printf("Markdown: %s\n", md);
        pdftract_free(md);
    }
    
    // Test null handling
    char* null_result = pdftract_extract(NULL, "{}");
    if (null_result) {
        printf("Null test: %s\n", null_result);
        pdftract_free(null_result);
    }
    
    printf("\nAll API calls completed!\n");
    return 0;
}
