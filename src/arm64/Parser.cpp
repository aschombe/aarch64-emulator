#include "arm64/Parser.h"
#include "core/CoreTypes.h"

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
    if (result != (size_t)lSize) {
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

char* processLine(char* pLine) {
    if (!pLine || !*pLine) return pLine;

    // Removes leading and trailing spaces (from around commas and brackets)
    char* pSrc = pLine;
    char* pDest = pLine;
    while (*pSrc) {
        if (*pSrc == ' ' && (*(pSrc + 1) == ',' || *(pSrc + 1) == '[' || *(pSrc + 1) == ']')) {
            pSrc++;
            continue;
        }
        if ((pDest > pLine && (*(pDest - 1) == ',' || *(pDest - 1) == '[' || *(pDest - 1) == ']')) && *pSrc == ' ') {
            pSrc++;
            continue;
        }
        *pDest++ = *pSrc++;
    }
    *pDest = '\0';

    // Remove brackets and replace commas with spaces
    pSrc = pLine;
    pDest = pLine;
    while (*pSrc) {
        // Skip brackets
        if (*pSrc == '[' || *pSrc == ']') {
            pSrc++;
            continue;
        }
        if (*pSrc == ',') {
            // Replace comma with space
            *pDest++ = ' ';  
        } else {
            *pDest++ = *pSrc;
        }
        pSrc++;
    }
    *pDest = '\0';

    // Iterate through and convert to lowercase (skipping labels and directives)
    pSrc = pLine;
    char* pTemp = pLine;
    pDest = pLine;
    while (*pSrc) {
        // Look ahead with pTemp to see if there are any colons
        pTemp = pSrc;
        while (*pTemp && *pTemp != ' ' && *pTemp != '\0') {
            if (*pTemp == ':') {
                break;
            }
            pTemp++;
        }
        if (*pTemp == ':') {
            // Skip this token
            while (*pSrc && *pSrc != ' ' && *pSrc != '\0') {
                *pDest = *pSrc;
                pSrc++;
                pDest++;
            }
            *pDest = *pSrc;
            pSrc++;
            pDest++;
            continue;
        }

        // Look ahead with pTemp to see if there are any dots
        pTemp = pSrc;
        while (*pTemp && *pTemp != ' ' && *pTemp != '\0') {
            if (*pTemp == '.') {
                break;
            }
            pTemp++;
        }
        if (*pTemp == '.') {
            // Skip this token
            while (*pSrc && *pSrc != ' ' && *pSrc != '\0') {
                *pDest = *pSrc;
                pSrc++;
                pDest++;
            }
            *pDest = *pSrc;
            pSrc++;
            pDest++;
            continue;
        }

        // Convert to lowercase
        if (*pSrc != ' ' && *pSrc != '\0') {
            *pDest = tolower(*pSrc);
        } else {
            *pDest = *pSrc;
        }
        pSrc++;
        pDest++;
    }

    return pLine;
}

char* pTokenToString(Token* pToken) {
    if (pToken == NULL) {
        return NULL;
    }

    char buffer[256];
    sprintf(buffer, "Token: %s, Line: %d, Col: %d", pToken->pValue, pToken->nLine, pToken->nCol);
    return strdup(buffer);
}

Token** Parse(char* pFile) {
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

    // Count the number of lines
    int nLines = 0;
    for (char* p = pFileContents; *p; p++) {
        if (*p == '\n') nLines++;
    }

    // Allocate memory for the Token array
    Token** pNodes = (Token**)malloc((nLines + 1) * sizeof(Token*));
    if (pNodes == NULL) {
        free(pFileContents);
        return NULL;
    }

    char* pLine = pFileContents;
    int lineNum = 1;
    int lineIndex = 0;
    
    while (*pLine) {
        char* pEnd = strchr(pLine, '\n');
        if (pEnd) *pEnd = '\0';
        
        processLine(pLine);
        
        // Tokenize line
        int capacity = 10;
        int tokenIndex = 0;
        Token* tokens = (Token*)malloc(capacity * sizeof(Token));
        
        char* pToken = strtok(pLine, " ");
        int colNum = 1;
        
        while (pToken) {
            if (tokenIndex >= capacity) {
                capacity *= 2;
                tokens = (Token*)realloc(tokens, capacity * sizeof(Token));
            }
            
            tokens[tokenIndex].pValue = strdup(pToken);
            tokens[tokenIndex].nLine = lineNum;
            tokens[tokenIndex].nCol = colNum;
            tokenIndex++;
            
            colNum += strlen(pToken) + 1;
            pToken = strtok(NULL, " ");
        }
        
        // Null terminate the token array
        tokens[tokenIndex].pValue = NULL;
        pNodes[lineIndex++] = tokens;
        
        if (pEnd) pLine = pEnd + 1;
        else break;
        lineNum++;
    }
    
    // Null terminate the line array
    pNodes[lineIndex] = NULL;
    free(pFileContents);
    return pNodes;
}
