// test_ffi.cpp - C++ smoke test for the rustiniparser C ABI.
//
// Confirms the header is usable from C++ (extern "C" linkage) and that a
// std::string-based round-trip through the buffer getter works.
#include "rustiniparser.h"

#include <cassert>
#include <cstdio>
#include <string>

static std::string get(IniDocument *d, const char *s, const char *k, const char *fb) {
    size_t n = ini_get_string(d, s, k, fb, nullptr, 0);
    std::string out(n, '\0');
    ini_get_string(d, s, k, fb, &out[0], n + 1);
    return out;
}

int main() {
    IniDocument *doc = ini_parse("[net]\nhost = example.com\nport = 8080\non = yes\n");
    assert(doc != nullptr);

    assert(get(doc, "net", "host", "?") == "example.com");
    assert(ini_get_int(doc, "net", "port", -1) == 8080);
    assert(ini_get_bool(doc, "net", "on", 0) == 1);
    assert(ini_len(doc) == 3);

    assert(ini_set_int(doc, "net", "port", 9090) == INI_OK);
    assert(ini_get_int(doc, "net", "port", -1) == 9090);

    assert(get(doc, "net", "missing", "fallback") == "fallback");

    ini_free(doc);
    std::printf("all C++ FFI checks passed\n");
    return 0;
}
