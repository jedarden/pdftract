#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>
#include "../crates/pdftract-libpdftract/include/pdftract.h"

int main(void) {
    printf("Testing stream API with tests/fixtures/test-minimal.pdf...\n");
    
    void* handle = pdftract_extract_stream_open("tests/fixtures/test-minimal.pdf", "{}");
    if (handle == NULL) {
        const char* error = pdftract_last_error();
        printf("Stream open failed: %s\n", error ? error : "(null)");
        return 1;
    }
    
    printf("Stream opened successfully\n");
    
    int page_count = 0;
    char* page;
    while ((page = pdftract_stream_next(handle)) != NULL) {
        page_count++;
        printf("Page %d: %zu bytes\n", page_count, strlen(page));
        pdftract_free(page);
    }
    
    pdftract_stream_close(handle);
    printf("Stream closed: %d pages\n", page_count);
    
    return 0;
}
