#include <stdio.h>
#include "/home/coding/pdftract/crates/pdftract-libpdftract/include/pdftract.h"

int main() {
    char *result = pdftract_extract_text("tests/fixtures/valid-minimal.pdf", "{}");
    printf("Result: %s\n", result ? result : "NULL");
    if (result) pdftract_free(result);
    
    const char *err = pdftract_last_error();
    printf("Last error: %s\n", err ? err : "none");
    
    return 0;
}
