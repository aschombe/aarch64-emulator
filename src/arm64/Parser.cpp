#include "arm64/Parser.h"

char* readFile(const char* pFile) {
    FILE* pF = fopen(pFile, "r");
    if (pF == NULL) {
        return NULL;
    }

    fseek(pF, 0, SEEK_END);
    long lSize = ftell(pF);
    rewind(pF);

    char* pStr = (char*)malloc(lSize + 1);
    if (pStr == NULL) {
        return NULL;
    }

    size_t result = fread(pStr, 1, lSize, pF);
    if (result != lSize) {
        return NULL;
    }

    fclose(pF);
    pStr[lSize] = '\0';
    return pStr;
}

char* cleanFileContents(char* pFileContents) {
    // Remove single line comments
    char* pCommentStart = strstr(pFileContents, "//");
    while (pCommentStart != NULL) {
        char* pCommentEnd = strstr(pCommentStart, "\n");
        if (pCommentEnd == NULL) {
            pCommentEnd = pCommentStart + strlen(pCommentStart);
        }
        memset(pCommentStart, '\n', pCommentEnd - pCommentStart);
        pCommentStart = strstr(pCommentEnd, "//");
    }

    // Remove multi-line comments
    pCommentStart = strstr(pFileContents, "/*");
    while (pCommentStart != NULL) {
        char* pCommentEnd = strstr(pCommentStart, "*/");
        if (pCommentEnd == NULL) {
            pCommentEnd = pCommentStart + strlen(pCommentStart);
        }
        memset(pCommentStart, '\n', pCommentEnd - pCommentStart + 2);
        pCommentStart = strstr(pCommentEnd, "/*");
    }

    // Remove empty lines
    char* pDest = pFileContents;
    char* pSrc = pFileContents;
    while (*pSrc) {
        if (*pSrc == '\n') {
            if (*(pSrc + 1) == '\n') {
                pSrc++;
                continue;
            }
        }
        *pDest = *pSrc;
        pDest++;
        pSrc++;
    }
    *pDest = '\0';

    return pFileContents;
}

// Converts a string to lowercase in-place (only for letters, skips numbers, punctuation, etc.)
void strLwr(char* pStr) {
    if (pStr == NULL) {
        return;
    }

    while (*pStr) {
        if (isalpha((unsigned char)*pStr)) {
            *pStr = tolower((unsigned char)*pStr);
        }
        pStr++;
    }
}

ASTNode** Parse(char* pFile) {
    // Read file contents
    char* pFileContents = readFile(pFile);
    if (pFileContents == NULL) {
        return NULL;
    }

    // Clean file contents
    pFileContents = cleanFileContents(pFileContents);
    if (pFileContents == NULL) {
        return NULL;
    }

    // Split file contents into lines and store in array
    char* pLine = strtok(pFileContents, "\n");

    // Count the number of lines (for approximating the size of the ASTNode array)
    int nLines = 0;
    while (pLine != NULL) {
        nLines++;
        pLine = strtok(NULL, "\n");
    }

    // Allocate memory for the ASTNode array (each line can have up to 1 label, 1 instruction and 3 operands)
    ASTNode** pNodes = (ASTNode**)malloc(nLines * (sizeof(Label) + sizeof(Instruction) + 3 * sizeof(Operand)));
    if (pNodes == NULL) {
        return NULL;
    }

    // Parse each line
    // pLine = strtok(pFileContents, "\n");
    // int nLine = 0;
    // int nCol = 0;
    // while (pLine != NULL) {

    // }

    return NULL;
}
