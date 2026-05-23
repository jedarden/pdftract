#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "../../crates/pdftract-libpdftract/include/pdftract.h"

int main(void) {
    const char *pdf_path = "/tmp/test.pdf";
    
    // Create minimal PDF
    const char *pdf_data =
        "%PDF-1.4\n"
        "1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n"
        "2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj\n"
        "3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]/Resources<</Font<</F1<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>>>>>>>>>endobj\n"
        "xref\n"
        "0 4\n"
        "0000000000 65535 f\n"
        "0000000009 00000 n\n"
        "0000000052 00000 n\n"
        "0000000109 00000 n\n"
        "trailer<</Size 4/Root 1 0 R>>\n"
        "startxref\n"
        "206\n"
        "%%EOF\n";

    FILE *f = fopen(pdf_path, "w");
    fwrite(pdf_data, 1, strlen(pdf_data), f);
    fclose(f);

    // Test hash function
    char *result = pdftract_hash(pdf_path);
    if (result) {
        printf("Hash result: %s\n", result);
        pdftract_free(result);
    } else {
        printf("Hash returned null\n");
    }

    // Test extract function
    result = pdftract_extract(pdf_path, "{}");
    if (result) {
        printf("Extract result (first 500 chars): %.500s...\n", result);
        pdftract_free(result);
    } else {
        printf("Extract returned null\n");
    }

    return 0;
}
