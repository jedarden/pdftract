/* Copyright 2026 Jed Cabanino. MIT OR Apache-2.0 */

/*
 * Sample C++ client for pdftract library.
 * Demonstrates C++ compatibility (using extern "C").
 */

#include <iostream>
#include <string>
#include <memory>
#include "../../crates/pdftract-libpdftract/include/pdftract.h"

/* RAII wrapper for pdftract strings */
struct PdftractString {
    char* ptr;

    PdftractString(char* p) : ptr(p) {}
    ~PdftractString() { if (ptr) pdftract_free(ptr); }

    // Disable copy
    PdftractString(const PdftractString&) = delete;
    PdftractString& operator=(const PdftractString&) = delete;

    // Enable move
    PdftractString(PdftractString&& other) noexcept : ptr(other.ptr) {
        other.ptr = nullptr;
    }
    PdftractString& operator=(PdftractString&& other) noexcept {
        if (this != &other) {
            if (ptr) pdftract_free(ptr);
            ptr = other.ptr;
            other.ptr = nullptr;
        }
        return *this;
    }

    std::string_view view() const {
        return ptr ? std::string_view(ptr) : std::string_view();
    }

    explicit operator bool() const { return ptr != nullptr; }
};

int main() {
    std::cout << "pdftract C++ client test\n";
    std::cout << "========================\n\n";

    // Test version
    std::cout << "Version: " << pdftract_version() << "\n\n";

    // Test null handling
    std::cout << "Testing null source handling...\n";
    PdftractString null_result(pdftract_extract(nullptr, "{}"));
    if (null_result && null_result.view().find("\"error\"") != std::string_view::npos) {
        std::cout << "PASS: null source returns error JSON\n";
    } else {
        std::cout << "FAIL: null source did not return error JSON\n";
    }

    std::cout << "\nAll C++ client tests completed.\n";
    return 0;
}
