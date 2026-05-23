#include <stdio.h>
#include "../crates/pdftract-libpdftract/include/pdftract.h"

int main(void) {
    printf("Testing pdftract_version...\n");
    const char* version = pdftract_version();
    printf("Version: %s\n", version);
    
    printf("Testing pdftract_abi_version...\n");
    uint32_t abi = pdftract_abi_version();
    printf("ABI: 0x%08x\n", abi);
    
    printf("Testing pdftract_free with NULL...\n");
    pdftract_free(NULL);
    printf("All tests passed!\n");
    return 0;
}
