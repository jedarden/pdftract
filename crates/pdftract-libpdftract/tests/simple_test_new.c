#include <stdio.h>
#include "../include/pdftract.h"

int main() {
    const char *version = pdftract_version();
    printf("Version: %s\n", version);
    
    uint32_t abi = pdftract_abi_version();
    printf("ABI Version: 0x%08x\n", abi);
    
    // Test hash with a simple file
    char *result = pdftract_hash("valid_test.pdf");
    if (result == NULL) {
        printf("Hash returned NULL\n");
        const char *err = pdftract_last_error();
        if (err) printf("Error: %s\n", err);
    } else {
        printf("Hash result: %s\n", result);
        pdftract_free(result);
    }
    
    return 0;
}
