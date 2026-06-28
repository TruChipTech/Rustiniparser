/*
 * sample.c - using rustiniparser from C.
 *
 * Build (after `cargo build --release` from the repo root):
 *   gcc -I../../include sample.c ../../target/release/librustiniparser.a \
 *       -lpthread -ldl -lm -o sample_c
 * or with the shared library:
 *   gcc -I../../include sample.c -L../../target/release -lrustiniparser -o sample_c
 *   LD_LIBRARY_PATH=../../target/release ./sample_c
 */
#include "rustiniparser.h"
#include <stdio.h>

static const char *CONFIG =
    "; application config\n"
    "app_name = Rustiniparser\n"
    "[network]\n"
    "host = \"example.com\"   ; inline comment\n"
    "port = 8080\n"
    "ratio = 0.25\n"
    "mask = 0xFF\n"
    "[server]\n"
    "enabled = yes\n";

int main(void) {
    IniDocument *doc = ini_parse(CONFIG);
    if (!doc) {
        fprintf(stderr, "failed to parse config\n");
        return 1;
    }

    /* Read values back, converted to the type we want. Note: "" is the
       global section (keys before any [header]). */
    char app[64];
    ini_get_string(doc, "", "app_name", "?", app, sizeof app);

    char host[128];
    ini_get_string(doc, "network", "host", "localhost", host, sizeof host);

    printf("app_name      = %s\n", app);
    printf("network.host  = %s\n", host);
    printf("network.port  = %lld\n", ini_get_int(doc, "network", "port", -1));
    printf("network.mask  = %lld (0xFF)\n", ini_get_int(doc, "network", "mask", -1));
    printf("network.ratio = %g\n", ini_get_double(doc, "network", "ratio", 0.0));
    printf("server.enabled= %d\n", ini_get_bool(doc, "server", "enabled", 0));
    printf("entries       = %zu\n", ini_len(doc));

    /* Modify an existing value and add a new one. */
    ini_set_int(doc, "network", "port", 9090);
    ini_set(doc, "network", "gateway", "10.0.0.1");

    char gw[64];
    ini_get_string(doc, "network", "gateway", "?", gw, sizeof gw);
    printf("\nafter edits:\n");
    printf("network.port    = %lld\n", ini_get_int(doc, "network", "port", -1));
    printf("network.gateway = %s\n", gw);
    printf("entries         = %zu\n", ini_len(doc));

    ini_free(doc);
    return 0;
}
