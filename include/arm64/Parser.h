#pragma once

#define NEWLINE_TOKEN_VALUE "__NEWLINE__"

typedef struct Token {
    char *pValue;
    int nLine;
    int nCol;
} Token;

char *pTokenToString(Token *pToken);

Token **Parse(char *pFile);

char *processLine(char *pline);

char *readFile(const char *pFile);

char *cleanFileContents(char *pFileContents);

void strLwr(char *pStr);

void freeTokens(Token **pNodes);

void printTokens(Token **pTokens);

void printToken(Token *pToken);

char *preprocessFile(const char *pFile);
