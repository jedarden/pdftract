#include <stdio.h>
#include <stdlib.h>
#include "../include/pdftract.h"

int main() {
    const char *path = "/home/coding/pdftract/tests/fixtures/valid-minimal.pdf";
    printf("Testing pdftract_hash with: %s\n", path);
    
    char *result = pdftract_hash(path);
    if (result == NULL) {
        const char *err = pdftract_last_error();
        printf("pdftract_hash returned NULL\n");
        printf("last_error: %s\n", err ? err : "(null)");
        return 1;
    }
    
    printf("Result: %s\n", result);
    pdftract_free(result);
    return 0;
}
