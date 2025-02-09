#pragma once

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>

typedef struct Token {
    char* pValue;
    int nLine;
    int nCol;
} Token;

char* pTokenToString(Token* pToken);

Token** Parse(char* pFile);

char* processLine(char* pline);

char* readFile(const char* pFile);

char* cleanFileContents(char* pFileContents);

void strLwr(char* pStr);