#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "../../crates/pdftract-libpdftract/include/pdftract.h"

int main(void) {
    const char *pdf_path = "/tmp/test_stream.pdf";
    FILE *f = fopen(pdf_path, "w");
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
    fwrite(pdf_data, 1, strlen(pdf_data), f);
    fclose(f);

    printf("Opening stream...\n");
    void *handle = pdftract_extract_stream_open(pdf_path, "{}");
    printf("Handle: %p\n", (void*)handle);

    if (handle == NULL) {
        printf("Failed to open stream\n");
        return 1;
    }

    printf("Getting first page...\n");
    char *page = pdftract_stream_next(handle);
    printf("Page: %p\n", (void*)page);

    if (page) {
        printf("Page content: %.100s\n", page);
        pdftract_free(page);
    }

    printf("Closing stream...\n");
    pdftract_stream_close(handle);

    remove(pdf_path);
    printf("Done\n");
    return 0;
}
