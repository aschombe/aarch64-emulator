#include "arm64/Parser.h"

#include <ctype.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

char *readFile(const char *pFile) {
    FILE *pF = fopen(pFile, "r");
    if (!pF)
        return NULL;

    fseek(pF, 0, SEEK_END);
    long lSize = ftell(pF);
    rewind(pF);

    char *pStr = (char *)malloc((size_t)lSize + 1);
    if (!pStr)
        return NULL;

    fread(pStr, 1, (size_t)lSize, pF);
    fclose(pF);
    pStr[lSize] = '\0';
    return pStr;
}

char *cleanFileContents(char *pFileContents) {
    char *pSrc = pFileContents;
    char *pDest = pFileContents;
    int nestedCount = 0;

    while (*pSrc) {
        // Handle block comments (/* ... */)
        if (*pSrc == '/' && *(pSrc + 1) == '*') {
            nestedCount++;
            pSrc += 2;
        } else if (*pSrc == '*' && *(pSrc + 1) == '/' && nestedCount > 0) {
            nestedCount--;
            pSrc += 2;
            continue;
        }
        // Handle single-line comments (// ...)
        else if (*pSrc == '/' && *(pSrc + 1) == '/') {
            while (*pSrc && *pSrc != '\n')
                pSrc++; // Skip until end of line
            continue;
        }

        if (nestedCount == 0)
            *pDest++ = *pSrc;
        pSrc++;
    }
    *pDest = '\0';

    return pFileContents;
}

void strLwr(char *pStr) {
    while (*pStr) {
        if (isalpha((unsigned char)*pStr)) {
            *pStr = (char)tolower((unsigned char)*pStr);
        }
        pStr++;
    }
}

char *processLine(char *pLine) {
    char *pSrc = pLine;
    char *pDest = pLine;

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

// Returns the cleaned file contents directly for Parse to tokenize
char *preprocessFile(const char *pFile) {
    char *pFileContents = readFile(pFile);
    if (!pFileContents)
        return NULL;

    pFileContents = cleanFileContents(pFileContents);
    if (!pFileContents)
        return NULL;

    return pFileContents;
}

Token **Parse(char *pFile) {
    // Call preprocessFile, which now returns the cleaned buffer directly
    char *pFileContents = preprocessFile(pFile);
    if (!pFileContents)
        return NULL;

    int capacity = 100;
    int tokenIndex = 0;
    Token **pTokens =
        (Token **)malloc((unsigned long)capacity * sizeof(Token *));
    if (!pTokens) {
        free(pFileContents);
        return NULL;
    }

    // Initialize allocated token pointers to NULL for safer cleanup
    for (int i = 0; i < capacity; ++i)
        pTokens[i] = NULL;

    int lineNum = 1, colNum = 1;
    char *pSrc = pFileContents;

    while (*pSrc) {
        // Handle newlines: Create a special token
        if (*pSrc == '\n') {
            if (tokenIndex >= capacity) {
                capacity *= 2;
                pTokens = (Token **)realloc(pTokens, (unsigned int)capacity *
                                                         sizeof(Token *));
                if (!pTokens) {
                    // Cleanup previously allocated tokens
                    for (int i = 0; i < tokenIndex; ++i) {
                        free(pTokens[i]->pValue);
                        free(pTokens[i]);
                    }
                    free(pFileContents);
                    return NULL;
                }
                // Initialize newly allocated pointers to NULL
                for (int i = tokenIndex; i < capacity; ++i)
                    pTokens[i] = NULL;
            }
            Token *newToken = (Token *)malloc(sizeof(Token));
            if (!newToken) {
                // Cleanup previously allocated tokens and the array
                for (int i = 0; i < tokenIndex; ++i) {
                    free(pTokens[i]->pValue);
                    free(pTokens[i]);
                }
                free(pTokens);
                free(pFileContents);
                return NULL;
            }
            newToken->pValue = strdup(NEWLINE_TOKEN_VALUE);
            if (!newToken->pValue) {
                // Cleanup previously allocated tokens and the array
                for (int i = 0; i < tokenIndex; ++i) {
                    free(pTokens[i]->pValue);
                    free(pTokens[i]);
                }
                free(newToken);
                free(pTokens);
                free(pFileContents);
                return NULL;
            }
            newToken->nLine = lineNum;
            newToken->nCol =
                colNum; // The column where the newline char was found
            pTokens[tokenIndex++] = newToken;

            lineNum++;
            colNum = 1;
            pSrc++;   // Move past the newline character
            continue; // Go to the next iteration
        }

        // Skip other whitespace (spaces, tabs) but track column properly
        while (*pSrc == ' ' || *pSrc == '\t') {
            colNum++;
            pSrc++;
        }

        // If we are at the end of the file after processing
        // whitespace/newlines, continue
        if (*pSrc == '\0') {
            continue;
        }

        // Start of a token (non-whitespace, non-newline)
        char *start = pSrc;
        int startCol = colNum;

        // Identify token boundaries
        while (*pSrc && *pSrc != ' ' && *pSrc != '\t' && *pSrc != '\n') {
            colNum++;
            pSrc++;
        }

        // Allocate and store the token
        int length = (int)(pSrc - start);
        if (length > 0) {
            if (tokenIndex >= capacity) {
                capacity *= 2;
                pTokens = (Token **)realloc(pTokens, (unsigned int)capacity *
                                                         sizeof(Token *));
                if (!pTokens) {
                    // Cleanup previously allocated tokens
                    for (int i = 0; i < tokenIndex; ++i) {
                        free(pTokens[i]->pValue);
                        free(pTokens[i]);
                    }
                    free(pFileContents);
                    return NULL;
                }
                // Initialize newly allocated pointers to NULL
                for (int i = tokenIndex; i < capacity; ++i)
                    pTokens[i] = NULL;
            }

            Token *newToken = (Token *)malloc(sizeof(Token));
            if (!newToken) {
                // Cleanup previously allocated tokens and the array
                for (int i = 0; i < tokenIndex; ++i) {
                    free(pTokens[i]->pValue);
                    free(pTokens[i]);
                }
                free(pTokens);
                free(pFileContents);
                return NULL;
            }
            newToken->pValue = strndup(start, (unsigned long)length);
            if (!newToken->pValue) {
                // Cleanup previously allocated tokens and the array
                for (int i = 0; i < tokenIndex; ++i) {
                    free(pTokens[i]->pValue);
                    free(pTokens[i]);
                }
                free(newToken);
                free(pTokens);
                free(pFileContents);
                return NULL;
            }
            newToken->nLine = lineNum;
            newToken->nCol = startCol;
            pTokens[tokenIndex++] = newToken;
        }
        // If length is 0, it means we encountered two delimiters in a row
        // (e.g., space then newline), pSrc is already advanced by the
        // whitespace/newline handling, so we just loop.
    }

    pTokens[tokenIndex] = NULL; // Null-terminate the token array

    // Free the buffer returned by preprocessFile
    free(pFileContents);
    return pTokens;
}

void freeTokens(Token **pNodes) {
    if (!pNodes)
        return;

    for (int i = 0; pNodes[i] != NULL; i++) {
        free(pNodes[i]->pValue); // Free the string value
        free(pNodes[i]);         // Free the Token struct
    }
    free(pNodes); // Free the array of Token pointers
}

void printTokens(Token **pTokens) {
    if (!pTokens) {
        return;
    }

    for (int i = 0; pTokens[i] != NULL; i++) {
        printToken(pTokens[i]);
    }
}

void printToken(Token *pToken) {
    if (!pToken) {
        return;
    }
    if (strcmp(pToken->pValue, NEWLINE_TOKEN_VALUE) == 0) {
        printf("Token: NEWLINE, Line: %d, Col: %d\n", pToken->nLine,
               pToken->nCol);
    } else {
        printf("Token: %s, Line: %d, Col: %d\n", pToken->pValue, pToken->nLine,
               pToken->nCol);
    }
}

char *pTokenToString(Token *pToken) {
    if (!pToken)
        return NULL;

    char buffer[256];
    if (strcmp(pToken->pValue, NEWLINE_TOKEN_VALUE) == 0) {
        snprintf(buffer, sizeof(buffer), "Token: NEWLINE, Line: %d, Col: %d",
                 pToken->nLine, pToken->nCol);
    } else {
        snprintf(buffer, sizeof(buffer), "Token: %s, Line: %d, Col: %d",
                 pToken->pValue, pToken->nLine, pToken->nCol);
    }
    return strdup(buffer);
}
