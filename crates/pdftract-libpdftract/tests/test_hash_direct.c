#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "../include/pdftract.h"

int main(int argc, char *argv[]) {
    const char *pdf_path = "../../../tests/fixtures/valid-minimal.pdf";
    if (argc > 1) {
        pdf_path = argv[1];
    }

    printf("Testing pdftract_hash with: %s\n", pdf_path);

    char *result = pdftract_hash(pdf_path);
    if (result == NULL) {
        const char *err = pdftract_last_error();
        printf("ERROR: pdftract_hash returned NULL\n");
        printf("Last error: %s\n", err ? err : "(null)");
        return 1;
    }

    printf("Result: %s\n", result);

    if (strstr(result, "\"fingerprint\"") == NULL) {
        printf("FAIL: result does not contain fingerprint field\n");
        pdftract_free(result);
        return 1;
    }

    printf("PASS: fingerprint found\n");
    pdftract_free(result);
    return 0;
}
