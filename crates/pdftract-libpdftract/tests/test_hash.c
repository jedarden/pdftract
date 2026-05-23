#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "../include/pdftract.h"

int main() {
    printf("Testing pdftract library...\n");
    
    // Test version
    const char *version = pdftract_version();
    printf("Version: %s\n", version);
    
    // Test ABI version
    uint32_t abi = pdftract_abi_version();
    printf("ABI Version: 0x%08x\n", abi);
    
    // Test hash
    char *result = pdftract_hash("valid-test.pdf");
    if (result == NULL) {
        const char *err = pdftract_last_error();
        printf("Hash failed (NULL result). Last error: %s\n", err ? err : "none");
        return 1;
    }
    
    printf("Hash result: %s\n", result);
    pdftract_free(result);
    
    return 0;
}
