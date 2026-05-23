#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* Create a minimal valid PDF with proper trailer and content stream */
int create_valid_pdf(const char* path) {
    FILE* f = fopen(path, "wb");
    if (!f) return 1;
    
    /* A valid minimal PDF with proper trailer and content stream */
    const char* pdf_content =
        "%PDF-1.4\n"
        "1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n"
        "2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj\n"
        "3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]"
        "/Resources<</Font<</F1 4 0 R>>>>/Contents 5 0 R>>endobj\n"
        "4 0 obj<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>endobj\n"
        "5 0 obj<</Length 44>>stream\n"
        "BT\n"
        "/F1 12 Tf\n"
        "50 700 Td\n"
        "(Hello World) Tj\n"
        "ET\n"
        "endstream\n"
        "endobj\n"
        "xref\n"
        "0 6\n"
        "0000000000 65535 f\n"
        "0000000009 00000 n\n"
        "0000000058 00000 n\n"
        "0000000115 00000 n\n"
        "0000000262 00000 n\n"
        "0000000331 00000 n\n"
        "trailer<</Size 6/Root 1 0 R>>\n"
        "startxref\n"
        "430\n"
        "%%EOF\n";
    
    fwrite(pdf_content, 1, strlen(pdf_content), f);
    fclose(f);
    return 0;
}

int main(void) {
    if (create_valid_pdf("/tmp/test-valid.pdf") != 0) {
        fprintf(stderr, "Failed to create PDF\n");
        return 1;
    }
    printf("Created /tmp/test-valid.pdf\n");
    return 0;
}
