#include <stdio.h>
#include <assert.h>
#include "../crates/pdftract-libpdftract/include/pdftract.h"

int main(void) {
    const char* test_pdf = "tests/fixtures/test-minimal.pdf";
    
    char* result = pdftract_hash(test_pdf);
    if (result) {
        printf("Hash result: %s\n", result);
        pdftract_free(result);
    }
    
    // Test stream
    void* handle = pdftract_extract_stream_open(test_pdf, "{}");
    printf("Stream handle: %p\n", handle);
    
    if (handle != NULL) {
        int page_count = 0;
        char* page;
        while ((page = pdftract_stream_next(handle)) != NULL) {
            page_count++;
            printf("Page %d: %zu bytes\n", page_count, strlen(page));
            pdftract_free(page);
        }
        pdftract_stream_close(handle);
        printf("Total pages: %d\n", page_count);
    } else {
        printf("Stream open returned NULL\n");
    }
    
    return 0;
}
