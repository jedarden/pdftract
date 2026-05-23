/* Create a minimal but valid PDF for testing */
#include <stdio.h>
#include <string.h>

int main() {
    FILE *f = fopen("valid-test.pdf", "wb");
    if (!f) return 1;
    
    /* A minimal valid PDF with a proper trailer */
    fprintf(f, "%%PDF-1.4\n");
    fprintf(f, "1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n");
    fprintf(f, "2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj\n");
    fprintf(f, "3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]");
    fprintf(f, "/Resources<</Font<</F1<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>>>>>");
    fprintf(f, "/Contents 4 0 R>>endobj\n");
    fprintf(f, "4 0 obj<</Length 44>>stream\n");
    fprintf(f, "BT\n/F1 12 Tf\n100 700 Td\n(Hello World) Tj\nET\n");
    fprintf(f, "endstream\nendobj\n");
    fprintf(f, "xref\n");
    fprintf(f, "0 5\n");
    fprintf(f, "0000000000 65535 f \n");
    fprintf(f, "0000000009 00000 n \n");
    fprintf(f, "0000000056 00000 n \n");
    fprintf(f, "0000000113 00000 n \n");
    fprintf(f, "0000000306 00000 n \n");
    fprintf(f, "trailer<</Size 5/Root 1 0 R>>\n");
    fprintf(f, "startxref\n");
    fprintf(f, "410\n");
    fprintf(f, "%%%%EOF\n");
    
    fclose(f);
    printf("Created valid-test.pdf\n");
    return 0;
}
