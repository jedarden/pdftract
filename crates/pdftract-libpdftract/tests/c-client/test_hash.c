int main() { char *r = pdftract_hash("/etc/passwd"); printf("Result: %s\n", r ? r : "NULL"); pdftract_free(r); return 0; }
