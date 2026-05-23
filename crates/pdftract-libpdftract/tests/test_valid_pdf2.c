#include <stdio.h>
#include "../include/pdftract.h"

int main() {
    char *result = pdftract_hash("/home/coding/pdftract/tests/fixtures/valid-minimal.pdf");
    if (result == NULL) {
        printf("Hash returned NULL\n");
        const char *err = pdftract_last_error();
        if (err) printf("Error: %s\n", err);
        return 1;
    } else {
        printf("Hash result: %s\n", result);
        if (strstr(result, "\"error\"") == NULL) {
            printf("SUCCESS: Got valid fingerprint\n");
            pdftract_free(result);
            return 0;
        }
        pdftract_free(result);
        return 1;
    }
}
