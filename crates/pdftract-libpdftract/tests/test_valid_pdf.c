#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "../include/pdftract.h"

int main() {
    const char *test_pdfs[] = {
        "/home/coding/pdftract/tests/fixtures/test-minimal.pdf",
        "valid_test.pdf",
        NULL
    };
    
    for (int i = 0; test_pdfs[i] != NULL; i++) {
        printf("Testing %s...\n", test_pdfs[i]);
        char *result = pdftract_hash(test_pdfs[i]);
        if (result == NULL) {
            printf("  -> NULL\n");
            const char *err = pdftract_last_error();
            if (err) printf("  Error: %s\n", err);
        } else {
            printf("  -> %s\n", result);
            if (strstr(result, "\"error\"") == NULL) {
                printf("  SUCCESS: Got valid fingerprint\n");
                pdftract_free(result);
                return 0;
            }
            pdftract_free(result);
        }
    }
    
    printf("All test PDFs failed\n");
    return 1;
}
