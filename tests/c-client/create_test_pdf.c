#include <stdio.h>
#include <stdlib.h>

/* Create a minimal valid PDF for testing */
int main(void) {
    FILE *f = fopen("/tmp/test_minimal.pdf", "wb");
    if (!f) return 1;
    
    /* Minimal valid PDF with actual text */
    fprintf(f, "%%PDF-1.4\n");
    fprintf(f, "1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n");
    fprintf(f, "2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj\n");
    fprintf(f, "3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]/Resources<</Font<</F1 4 0 R>>>>/Contents 5 0 R>>endobj\n");
    fprintf(f, "4 0 obj<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>endobj\n");
    fprintf(f, "5 0 obj<</Length 44>>stream\n");
    fprintf(f, "BT\n/F1 12 Tf\n100 700 Td\n(Hello World) Tj\nET\n");
    fprintf(f, "endstream\nendobj\n");
    fprintf(f, "xref\n");
    fprintf(f, "0 6\n");
    fprintf(f, "0000000000 65535 f \n");
    fprintf(f, "0000000009 00000 n \n");
    fprintf(f, "0000000058 00000 n \n");
    fprintf(f, "0000000115 00000 n \n");
    fprintf(f, "0000000262 00000 n \n");
    fprintf(f, "0000000313 00000 n \n");
    fprintf(f, "trailer<</Size 6/Root 1 0 R>>\n");
    fprintf(f, "startxref\n");
    fprintf(f, "403\n");
    fprintf(f, "%%%%EOF\n");
    
    fclose(f);
    return 0;
}
