#include <stdio.h>
#include <stdlib.h>
#include "/home/coding/pdftract/crates/pdftract-libpdftract/include/pdftract.h"

int main() {
    const char *path = "/tmp/valid-minimal.pdf";
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
