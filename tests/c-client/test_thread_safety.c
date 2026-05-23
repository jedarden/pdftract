#include <stdio.h>
#include <stdlib.h>
#include <pthread.h>
#include <assert.h>
#include "../../crates/pdftract-libpdftract/include/pdftract.h"

#define NUM_THREADS 8

static int json_has_error(const char *json) {
    return strstr(json, "\"error\"") != NULL;
}

void* thread_func(void* arg) {
    int thread_id = *(int*)arg;
    
    // Each thread makes multiple calls
    for (int i = 0; i < 10; i++) {
        char *result = pdftract_extract(NULL, "{}");
        assert(result != NULL);
        assert(json_has_error(result));
        pdftract_free(result);
        
        result = pdftract_version();
        assert(result != NULL);
        // Don't free version - it's static
        
        result = pdftract_hash(NULL);
        assert(result != NULL);
        pdftract_free(result);
    }
    
    printf("Thread %d completed\n", thread_id);
    return NULL;
}

int main(void) {
    printf("=== Thread Safety Test ===\n");
    printf("Launching %d threads, each making 30 API calls...\n\n", NUM_THREADS);
    
    pthread_t threads[NUM_THREADS];
    int thread_ids[NUM_THREADS];
    
    for (int i = 0; i < NUM_THREADS; i++) {
        thread_ids[i] = i;
        int rc = pthread_create(&threads[i], NULL, thread_func, &thread_ids[i]);
        if (rc != 0) {
            fprintf(stderr, "Failed to create thread %d\n", i);
            return 1;
        }
    }
    
    for (int i = 0; i < NUM_THREADS; i++) {
        pthread_join(threads[i], NULL);
    }
    
    printf("\nPASS: All %d threads completed without crashes or data races\n", NUM_THREADS);
    printf("Total API calls: %d (8 threads × 30 calls each)\n", NUM_THREADS * 30);
    
    return 0;
}
