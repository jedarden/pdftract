/* Copyright 2026 Jed Cabanino. MIT OR Apache-2.0 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "pdftract.h"

int main(void) {
    printf("=== Simple pdftract C API Test ===\n\n");

    // Test version
    printf("Version: %s\n", pdftract_version());
    printf("ABI Version: %u\n\n", pdftract_abi_version());

    // Test hash with absolute path
    const char *pdf_path = "/home/coding/pdftract/tests/c-client/fixtures/minimal.pdf";
    printf("Testing pdftract_hash with: %s\n", pdf_path);

    char *result = pdftract_hash(pdf_path);
    if (!result) {
        printf("ERROR: pdftract_hash returned NULL\n");
        return 1;
    }

    printf("Result: %s\n", result);

    if (strstr(result, "\"error\"")) {
        printf("ERROR: Got error response\n");
        pdftract_free(result);
        return 1;
    }

    pdftract_free(result);
    printf("\nTest passed!\n");
    return 0;
}
