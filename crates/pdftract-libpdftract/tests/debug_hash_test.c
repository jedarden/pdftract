#include <stdio.h>
#include <stdlib.h>
#include "../include/pdftract.h"

int main(int argc, char *argv[]) {
    if (argc < 2) {
        fprintf(stderr, "Usage: %s <pdf_path>\n", argv[0]);
        return 1;
    }

    const char *pdf_path = argv[1];
    printf("Testing pdftract_hash with: %s\n", pdf_path);

    char *result = pdftract_hash(pdf_path);
    if (result == NULL) {
        const char *err = pdftract_last_error();
        printf("pdftract_hash returned NULL\n");
        printf("last_error: %s\n", err ? err : "NULL");
        return 1;
    }

    printf("Result: %s\n", result);
    pdftract_free(result);
    return 0;
}
