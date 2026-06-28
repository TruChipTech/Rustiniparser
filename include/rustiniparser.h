/*
 * rustiniparser - C/C++ binding
 *
 * A tiny in-memory INI store. Link against librustiniparser.a (static) or
 * librustiniparser.so / .dll / .dylib (shared), both produced by
 * `cargo build --release`.
 *
 * Conventions:
 *   - An IniDocument* is opaque, created by ini_new()/ini_parse() and released
 *     exactly once with ini_free().
 *   - Use "" (empty string) for the global section.
 *   - String getters copy into a caller buffer; they never hand back a pointer
 *     into library-owned memory.
 *   - All string arguments are NUL-terminated, UTF-8, and only borrowed for the
 *     duration of the call.
 */
#ifndef RUSTINIPARSER_H
#define RUSTINIPARSER_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Opaque INI document handle. */
typedef struct IniDocument IniDocument;

/* Status codes. */
#define INI_OK            0
#define INI_ERR_SYNTAX    1
#define INI_ERR_TOO_LONG  2
#define INI_ERR_FULL      3
#define INI_ERR_NOT_FOUND 4
#define INI_ERR_NULL      5
#define INI_ERR_UTF8      6

/* ---- Lifecycle ---- */

/* New empty document. Free with ini_free(). */
IniDocument *ini_new(void);

/* Parse INI text into a new document. Returns NULL on NULL/invalid-UTF-8/
 * syntax error. Free with ini_free(). */
IniDocument *ini_parse(const char *text);

/* Merge more INI text into an existing document. Returns a status code. */
int ini_load(IniDocument *doc, const char *text);

/* Release a document. NULL is a no-op. Free each handle at most once. */
void ini_free(IniDocument *doc);

/* Number of stored entries (0 if doc is NULL). */
size_t ini_len(const IniDocument *doc);

/* ---- Getters ---- */

/* 1 if section/key exists, else 0. */
int ini_has(const IniDocument *doc, const char *section, const char *key);

/* Copy the value for section/key (or fallback if absent) into out[out_len].
 * Always NUL-terminates when out_len > 0. Returns the full value length
 * (excluding NUL); a return >= out_len means it was truncated. Pass
 * out=NULL,out_len=0 to query the required length. */
size_t ini_get_string(const IniDocument *doc, const char *section,
                      const char *key, const char *fallback,
                      char *out, size_t out_len);

/* Value as 64-bit int (decimal or 0x hex), or fallback. */
long long ini_get_int(const IniDocument *doc, const char *section,
                      const char *key, long long fallback);

/* Value as double, or fallback. */
double ini_get_double(const IniDocument *doc, const char *section,
                      const char *key, double fallback);

/* Value as bool (1/0), or fallback. */
int ini_get_bool(const IniDocument *doc, const char *section,
                 const char *key, int fallback);

/* ---- Setters ---- */

/* Set section/key to a string (add or overwrite). Status code. */
int ini_set(IniDocument *doc, const char *section, const char *key,
            const char *value);

/* Set section/key to an integer. Status code. */
int ini_set_int(IniDocument *doc, const char *section, const char *key,
                long long value);

/* Set section/key to a bool (value != 0). Status code. */
int ini_set_bool(IniDocument *doc, const char *section, const char *key,
                 int value);

/* Remove section/key. INI_OK or INI_ERR_NOT_FOUND. */
int ini_remove(IniDocument *doc, const char *section, const char *key);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* RUSTINIPARSER_H */
