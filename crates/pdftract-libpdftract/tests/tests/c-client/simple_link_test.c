#include <stdio.h>
#include <pdftract.h>

int main(void) {
    const char *version = pdftract_version();
    printf("pdftract version: %s\n", version);
    
    uint32_t abi = pdftract_abi_version();
    printf("ABI version: 0x%08x\n", abi);
    
    // Test that pdftract_free handles NULL
    pdftract_free(NULL);
    
    printf("Simple link test PASSED\n");
    return 0;
}
