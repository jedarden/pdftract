#include <stdio.h>
#include <assert.h>
#include "../crates/pdftract-libpdftract/include/pdftract.h"

int main(void) {
    const char* version = pdftract_version();
    printf("Version: %s\n", version);
    
    uint32_t abi = pdftract_abi_version();
    printf("ABI: 0x%08x\n", abi);
    
    char* result = pdftract_hash("/tmp/test.pdf");
    if (result) {
        printf("Hash result: %s\n", result);
        pdftract_free(result);
    }
    
    return 0;
}
