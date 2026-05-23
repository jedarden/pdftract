#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>
#include <pthread.h>
#include "../crates/pdftract-libpdftract/include/pdftract.h"

static const char* test_pdf_path = "/tmp/test-debug.pdf";

static void create_test_pdf(const char* path) {
    FILE* f = fopen(path, "wb");
    assert(f != NULL);
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
}

static void* thread_worker(void* arg) {
    for (int i = 0; i < 10; i++) {
        char* result = pdftract_hash(test_pdf_path);
        if (result != NULL) {
            pdftract_free(result);
        }
    }
    return NULL;
}

int main(void) {
    printf("=== Thread Safety Debug Test ===\n");
    
    create_test_pdf(test_pdf_path);
    
    printf("Testing thread safety...\n");
    pthread_t threads[4];
    for (int i = 0; i < 4; i++) {
        int rc = pthread_create(&threads[i], NULL, thread_worker, NULL);
        printf("  Created thread %d: rc=%d\n", i, rc);
        assert(rc == 0);
    }
    
    for (int i = 0; i < 4; i++) {
        printf("  Joining thread %d...\n", i);
        int rc = pthread_join(threads[i], NULL);
        printf("  Joined thread %d: rc=%d\n", i, rc);
        assert(rc == 0);
    }
    
    printf("[PASS] Thread safety test\n");
    
    remove(test_pdf_path);
    return 0;
}
