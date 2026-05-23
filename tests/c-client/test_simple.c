#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "../../crates/pdftract-libpdftract/include/pdftract.h"

int main(void) {
    printf("Testing version...\n");
    const char *version = pdftract_version();
    printf("Version: %s\n", version);
    
    printf("\nTesting hash...\n");
    char *result = pdftract_hash("/tmp/valid_test.pdf");
    if (result) {
        printf("Hash: %s\n", result);
        pdftract_free(result);
    }
    
    return 0;
}
