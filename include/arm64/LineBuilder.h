#pragma once

#include "arm64/TypeDecoder.h"

typedef struct Line {
    Symbol **symbols;
    int numSymbols;
    int lineNumber;
} Line;

Line **buildLines(Symbol **pSymbols);

void freeLines(Line **pLines);

void printLine(Line *pLine);

void printLines(Line **pLines);