#include "arm64/Parser.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>

char* readFile(const char* pFile) {
    FILE* pF = fopen(pFile, "r");
    if (!pF) return NULL;

    fseek(pF, 0, SEEK_END);
    long lSize = ftell(pF);
    rewind(pF);

    char* pStr = (char*)malloc((size_t)lSize + 1);
    if (!pStr) return NULL;

    fread(pStr, 1, (size_t) lSize, pF);
    fclose(pF);
    pStr[lSize] = '\0';
    return pStr;
}

char* cleanFileContents(char* pFileContents) {
    char* pSrc = pFileContents;
    char* pDest = pFileContents;
    int nestedCount = 0;

    while (*pSrc) {
        // Handle block comments (/* ... */)
        if (*pSrc == '/' && *(pSrc + 1) == '*') {
            nestedCount++;
            pSrc += 2;
        } 
        else if (*pSrc == '*' && *(pSrc + 1) == '/' && nestedCount > 0) {
            nestedCount--;
            pSrc += 2;
            continue;
        }
        // Handle single-line comments (// ...)
        else if (*pSrc == '/' && *(pSrc + 1) == '/') {
            while (*pSrc && *pSrc != '\n') pSrc++;  // Skip until end of line
            continue;
        }
        
        if (nestedCount == 0) *pDest++ = *pSrc;
        pSrc++;
    }
    *pDest = '\0';

    return pFileContents;
}


void strLwr(char* pStr) {
    while (*pStr) {
        if (isalpha((unsigned char)*pStr)) {
            *pStr = (char)tolower((unsigned char)*pStr);
        }
        pStr++;
    }
}

char* processLine(char* pLine) {
    char* pSrc = pLine;
    char* pDest = pLine;

    while (*pSrc) {
        if (*pSrc == '[' || *pSrc == ']') {
            pSrc++;
            continue;
        }
        if (*pSrc == ',') {
            *pDest++ = ' ';
        } else {
            *pDest++ = *pSrc;
        }
        pSrc++;
    }
    *pDest = '\0';
    strLwr(pLine);
    return pLine;
}

char* preprocessFile(const char* pFile) {
    char* pFileContents = readFile(pFile);
    if (!pFileContents) return NULL;

    pFileContents = cleanFileContents(pFileContents);
    if (!pFileContents) return NULL;

    char* pProcessed = (char*)malloc(strlen(pFileContents) * 2);
    if (!pProcessed) return NULL;

    pProcessed[0] = '\0';
    char* pLine = strtok(pFileContents, "\n");
    while (pLine) {
        processLine(pLine);
        strcat(pProcessed, pLine);
        strcat(pProcessed, "\n");
        pLine = strtok(NULL, "\n");
    }
    free(pFileContents);
    return pProcessed;
}

Token** Parse(char* pFile) {
    char* pFileContents = preprocessFile(pFile);
    if (!pFileContents) return NULL;

    int capacity = 100;
    int tokenIndex = 0;
    Token** pTokens = (Token**)malloc((unsigned long)capacity * sizeof(Token*));
    if (!pTokens) return NULL;

    int lineNum = 1, colNum = 1;
    char* pSrc = pFileContents;

    while (*pSrc) {
        // Skip whitespace but track column properly
        while (*pSrc == ' ' || *pSrc == '\t') {
            colNum++;
            pSrc++;
        }

        // Handle newlines properly
        if (*pSrc == '\n') {
            lineNum++;
            colNum = 1;
            pSrc++;
            continue;
        }

        // Start of a token
        char* start = pSrc;
        int startCol = colNum;

        // Identify token boundaries
        while (*pSrc && *pSrc != ' ' && *pSrc != '\t' && *pSrc != '\n') {
            colNum++;
            pSrc++;
        }

        // Allocate and store the token
        int length = (int) (pSrc - start);
        if (length > 0) {
            if (tokenIndex >= capacity) {
                capacity *= 2;
                pTokens = (Token**)realloc(pTokens, (unsigned int)capacity * sizeof(Token*));
            }

            Token* newToken = (Token*)malloc(sizeof(Token));
            newToken->pValue = strndup(start, (unsigned long)length);
            newToken->nLine = lineNum;
            newToken->nCol = startCol;
            pTokens[tokenIndex++] = newToken;
        }
    }

    pTokens[tokenIndex] = NULL;
    free(pFileContents);
    return pTokens;
}


void freeTokens(Token** pNodes) {
    if (!pNodes) return;

    for (int i = 0; pNodes[i] != NULL; i++) {
        free(pNodes[i]->pValue);
        free(pNodes[i]);
    }
    free(pNodes);
}

void printToken(Token* pToken) {
    if (!pToken) return;
    printf("Token: %s, Line: %d, Col: %d\n", pToken->pValue, pToken->nLine, pToken->nCol);
}

char* pTokenToString(Token* pToken) {
    if (!pToken) return NULL;

    char buffer[256];
    sprintf(buffer, "Token: %s, Line: %d, Col: %d", pToken->pValue, pToken->nLine, pToken->nCol);
    return strdup(buffer);
}

