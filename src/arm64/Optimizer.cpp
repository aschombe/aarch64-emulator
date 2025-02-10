#include "arm64/Optimizer.h"


Token** Optimize(Token** pTokens) {
    
}

// Ends with a colon
bool isLabel(Token* pToken) {
    return pToken->pValue[strlen(pToken->pValue) - 1] == ':';
}

// Starts with an 'x' or 'w' followed by a number between 0 and 31, or 'xzr' or 'wzr'
// Also supports 'sp', 'lr', 'nzcv' (future support for msr/mrs)
bool isRegister(Token* pToken) {
    if (strlen(pToken->pValue) < 2) {
        return false;
    }

    // Check for sp, lr, nzcv
    if (strcmp(pToken->pValue, "sp") == 0 || strcmp(pToken->pValue, "lr") == 0 || strcmp(pToken->pValue, "nzcv") == 0) {
        return true;
    }

    // Check for xzr, wzr
    if (strcmp(pToken->pValue, "xzr") == 0 || strcmp(pToken->pValue, "wzr") == 0) {
        return true;
    }

    // Check for x0-x31, w0-w31
    if (pToken->pValue[0] == 'x' || pToken->pValue[0] == 'w') {
        for (size_t i = 1; i < strlen(pToken->pValue); i++) {
            if (!isdigit(pToken->pValue[i])) {
                return false;
            }
        }

        int reg = atoi(pToken->pValue + 1);
        return reg >= 0 && reg <= 31;
    }

    return false;
}

// Checks for a number, multiple supported formats:
/*
(-)n
#(-)n
#0xn
#0bn
'c'
#'c'
*/
bool isImmediate(Token* pToken) {
    if (pToken->pValue[0] == '#') {
        if (pToken->pValue[1] == '0') {
            if (pToken->pValue[2] == 'x') {
                for (size_t i = 3; i < strlen(pToken->pValue); i++) {
                    if (!isxdigit(pToken->pValue[i])) {
                        return false;
                    }
                }
            } else if (pToken->pValue[2] == 'b') {
                for (size_t i = 3; i < strlen(pToken->pValue); i++) {
                    if (pToken->pValue[i] != '0' && pToken->pValue[i] != '1') {
                        return false;
                    }
                }
            } else {
                for (size_t i = 2; i < strlen(pToken->pValue); i++) {
                    if (!isdigit(pToken->pValue[i])) {
                        return false;
                    }
                }
            }
        } else {
            for (size_t i = 1; i < strlen(pToken->pValue); i++) {
                if (!isdigit(pToken->pValue[i])) {
                    return false;
                }
            }
        }

        return true;
    } else if (pToken->pValue[0] == '\'') {
        if (pToken->pValue[1] == '#') {
            if (pToken->pValue[2] == '\'') {
                return true;
            }
        } else {
            if (pToken->pValue[1] == '\'') {
                return true;
            }
        }
    }

    return false;
}

// Starts with a dot
bool isDirective(Token* pToken) {
    return pToken->pValue[0] == '.';
}