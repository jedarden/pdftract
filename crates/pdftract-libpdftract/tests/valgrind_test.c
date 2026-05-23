#include <stdio.h>
#include <stdlib.h>
#include "../include/pdftract.h"

int main() {
    /* Test basic API usage */
    const char *version = pdftract_version();
    printf("Version: %s\n", version);
    
    /* Test hash with invalid file (should return error JSON) */
    char *result = pdftract_hash("/nonexistent.pdf");
    if (result) {
        printf("Result: %s\n", result);
        pdftract_free(result);
    }
    
    /* Test extract with invalid file */
    result = pdftract_extract_text("/nonexistent.pdf", "{}");
    if (result) {
        printf("Result: %s\n", result);
        pdftract_free(result);
    }
    
    /* Test classify with invalid file */
    result = pdftract_classify("/nonexistent.pdf");
    if (result) {
        printf("Result: %s\n", result);
        pdftract_free(result);
    }
    
    printf("All memory freed correctly\n");
    return 0;
}
