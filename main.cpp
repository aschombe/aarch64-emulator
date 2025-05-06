#include <getopt.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "arm64/CFG.h"
#include "arm64/LineBuilder.h"
#include "arm64/Parser.h"
#include "arm64/TypeDecoder.h"

int main(int argc, char *argv[]) {
    char *help_message = (char *)malloc(256);
    snprintf(help_message, 256,
             "Usage: %s <filename> [-h] [-v]\n"
             "Flags:\n"
             "\t-h: Help\n"
             "\t-v: Version\n",
             argv[0]);

    // =============== Parse arguments ===============

    if (argc < 2) {
        printf("%s", help_message);
        free(help_message);
        return 1;
    }

    char *file = NULL;

    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "-h") == 0) {
            printf("%s", help_message);
            free(help_message);
            return 0;
        } else if (strcmp(argv[i], "-v") == 0) {
            printf("Version: 0.0.1\n");
            free(help_message);
            return 0;
        } else {
            file = argv[i];
        }
    }

    free(help_message);

    if (!file) {
        printf("No file provided\n");
        return 1;
    }

    if (strlen(file) < 2 || file[strlen(file) - 2] != '.' ||
        file[strlen(file) - 1] != 's') {
        printf("Please provide a valid .s file\n");
        return 1;
    }

    // =============== Parse file ===============

    Token **tokens = Parse(file);
    if (!tokens) {
        printf("Failed to parse file\n");
        freeTokens(tokens);
        return 1;
    }

    // =============== Decode types ===============

    Symbol **symbols = typeDecode(tokens);
    freeTokens(tokens);
    if (!symbols) {
        printf("Failed to decode types\n");
        freeSymbols(symbols);
        return 1;
    }

    // =============== Build lines ================

    Line **lines = buildLines(symbols);
    freeSymbols(symbols);
    if (!lines) {
        printf("Failed to build lines\n");
        freeLines(lines);
        return 1;
    }

    printLines(lines);

    freeLines(lines);

    // =============== Build CFG ===============

    /*CFG *cfg = buildCFG(symbols);*/
    /*freeSymbols(symbols);*/
    /*if (!cfg) {*/
    /*  printf("Failed to build CFG\n");*/
    /*  return 1;*/
    /*}*/
    /**/
    /*printf("CFG:\n");*/
    /*printCFG(cfg);*/
    /**/
    /*freeCFG(cfg);*/

    return 0;
}
