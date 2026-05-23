#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "../crates/pdftract-libpdftract/include/pdftract.h"

int main(void) {
    printf("Testing pdftract_extract_stream_open...\n");
    
    void* handle = pdftract_extract_stream_open("/tmp/test.pdf", "{}");
    printf("Handle: %p\n", handle);
    
    if (handle == NULL) {
        const char* error = pdftract_last_error();
        printf("Error: %s\n", error ? error : "(null)");
    } else {
        printf("Stream opened successfully\n");
        pdftract_stream_close(handle);
    }
    
    return 0;
}
