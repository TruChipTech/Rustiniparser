/*
 * test_ffi.c - assertion-based tests for the rustiniparser C ABI.
 *
 * Exercises parsing, typed getters, fallbacks, mutation, removal, buffer
 * truncation, and error/NULL handling. Exits non-zero on the first failure.
 */
#include "rustiniparser.h"

#include <assert.h>
#include <stdio.h>
#include <string.h>

static const char *SAMPLE =
    "; global config\n"
    "app_name = Rustiniparser\n"
    "[network]\n"
    "host = \"example.com\"   ; inline comment\n"
    "port = 8080\n"
    "ratio = 0.25\n"
    "mask = 0xFF\n"
    "[server]\n"
    "enabled = yes\n"
    "password = 'p@ss #word'\n";

static int tests_run = 0;
#define CHECK(cond)                                                          \
    do {                                                                     \
        tests_run++;                                                         \
        if (!(cond)) {                                                       \
            fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, #cond);  \
            return 1;                                                        \
        }                                                                    \
    } while (0)

static int gets(IniDocument *d, const char *s, const char *k, const char *fb,
                const char *expect) {
    char buf[256];
    ini_get_string(d, s, k, fb, buf, sizeof buf);
    return strcmp(buf, expect) == 0;
}

int main(void) {
    /* ---- parse + typed getters ---- */
    IniDocument *doc = ini_parse(SAMPLE);
    CHECK(doc != NULL);

    CHECK(gets(doc, "", "app_name", "?", "Rustiniparser"));
    CHECK(gets(doc, "network", "host", "?", "example.com"));      /* quotes/comment stripped */
    CHECK(gets(doc, "server", "password", "?", "p@ss #word"));    /* single-quoted, '#' kept */
    CHECK(ini_get_int(doc, "network", "port", -1) == 8080);
    CHECK(ini_get_int(doc, "network", "mask", -1) == 255);        /* 0xFF hex */
    double r = ini_get_double(doc, "network", "ratio", 0.0);
    CHECK(r > 0.249 && r < 0.251);
    CHECK(ini_get_bool(doc, "server", "enabled", 0) == 1);
    CHECK(ini_len(doc) == 7);

    /* ---- presence / fallbacks ---- */
    CHECK(ini_has(doc, "network", "host") == 1);
    CHECK(ini_has(doc, "network", "nope") == 0);
    CHECK(ini_get_int(doc, "network", "nope", 42) == 42);
    CHECK(gets(doc, "nope", "x", "def", "def"));

    /* ---- modify / add / remove ---- */
    size_t before = ini_len(doc);
    CHECK(ini_set_int(doc, "network", "port", 9090) == INI_OK);
    CHECK(ini_get_int(doc, "network", "port", 0) == 9090);
    CHECK(ini_len(doc) == before);                               /* overwrite, no growth */

    CHECK(ini_set(doc, "network", "gateway", "10.0.0.1") == INI_OK);
    CHECK(gets(doc, "network", "gateway", "?", "10.0.0.1"));
    CHECK(ini_len(doc) == before + 1);

    CHECK(ini_set_bool(doc, "server", "enabled", 0) == INI_OK);
    CHECK(ini_get_bool(doc, "server", "enabled", 1) == 0);

    CHECK(ini_remove(doc, "network", "port") == INI_OK);
    CHECK(ini_has(doc, "network", "port") == 0);
    CHECK(ini_remove(doc, "network", "port") == INI_ERR_NOT_FOUND);

    /* ---- ini_load merges ---- */
    CHECK(ini_load(doc, "[extra]\nk = v\n") == INI_OK);
    CHECK(gets(doc, "extra", "k", "?", "v"));

    /* ---- string buffer: truncation + required-length query ---- */
    char small[5];
    size_t need = ini_get_string(doc, "network", "gateway", "?", small, sizeof small);
    CHECK(need == 8);                       /* full length of "10.0.0.1" */
    CHECK(strcmp(small, "10.0") == 0);      /* truncated, still NUL-terminated */
    CHECK(ini_get_string(doc, "network", "gateway", "?", NULL, 0) == 8); /* query only */

    ini_free(doc);

    /* ---- error / NULL handling ---- */
    CHECK(ini_parse("[unterminated\n") == NULL);   /* syntax error -> NULL */
    CHECK(ini_parse("novalue\n") == NULL);
    CHECK(ini_set(NULL, "s", "k", "v") == INI_ERR_NULL);
    CHECK(ini_len(NULL) == 0);
    ini_free(NULL);                                 /* no-op, must not crash */

    printf("all %d C FFI checks passed\n", tests_run);
    return 0;
}
