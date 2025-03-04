#include "arm64/TypeDecoder.h"
#include "util/InstructionMap.h"

#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include <ctype.h>

// Decodes an array of tokens into an array of symbols
Symbol** TypeDecode(Token** pTokens) {
    size_t numTokens = 0;
    while (pTokens[numTokens] != NULL) numTokens++; // Count tokens

    Symbol** pSymbols = (Symbol**)malloc(sizeof(Symbol*) * (numTokens + 1));
    size_t symIndex = 0;

    for (size_t i = 0; i < numTokens; i++) {
        pSymbols[symIndex] = (Symbol*)malloc(sizeof(Symbol));

        // if the previous symbol was a .global directive, then this token is a label
        if (symIndex > 0 && pSymbols[symIndex - 1]->type == TokenType::DIRECTIVE) {
          if (strcmp((const char*)pSymbols[symIndex - 1]->directive.directive, ".global") == 0) {
            pSymbols[symIndex]->type = TokenType::LABEL;
            pSymbols[symIndex]->label.label = (U8*)malloc(strlen(pTokens[i]->pValue) + 1);
            strcpy((char*)pSymbols[symIndex]->label.label, pTokens[i]->pValue);
            pSymbols[symIndex]->nLine = pTokens[i]->nLine;
            pSymbols[symIndex]->nCol = pTokens[i]->nCol;
            symIndex++;
            continue;
          }
        }


        if (isLabel(pTokens[i])) {
            pSymbols[symIndex]->type = TokenType::LABEL;
            pSymbols[symIndex]->label.label = (U8*)malloc(strlen(pTokens[i]->pValue) + 1);
            strcpy((char*)pSymbols[symIndex]->label.label, pTokens[i]->pValue);
        } else if (isRegister(pTokens[i])) {
            pSymbols[symIndex]->type = TokenType::REGISTER;
            pSymbols[symIndex]->reg.reg = Reg::X0;
            if (pTokens[i]->pValue[0] == 'w') {
                pSymbols[symIndex]->reg.reg = (Reg)((int)Reg::W0 + atoi(pTokens[i]->pValue + 1));
            } else if (pTokens[i]->pValue[0] == 'x') {
                pSymbols[symIndex]->reg.reg = (Reg)((int)Reg::X0 + atoi(pTokens[i]->pValue + 1));
            } else if (strcmp(pTokens[i]->pValue, "sp") == 0) {
                pSymbols[symIndex]->reg.reg = Reg::SP;
            } else if (strcmp(pTokens[i]->pValue, "lr") == 0) {
                pSymbols[symIndex]->reg.reg = Reg::LR;
            } else if (strcmp(pTokens[i]->pValue, "wzr") == 0) {
                pSymbols[symIndex]->reg.reg = Reg::WZR;
            } else if (strcmp(pTokens[i]->pValue, "xzr") == 0) {
                pSymbols[symIndex]->reg.reg = Reg::XZR;
            } else if (strcmp(pTokens[i]->pValue, "nzcv") == 0) {
                pSymbols[symIndex]->reg.reg = Reg::NZCV;
            }
        } else if (isImmediate(pTokens[i])) {
            pSymbols[symIndex]->type = TokenType::IMMEDIATE;
            pSymbols[symIndex]->imm.value = 0;
            if (pTokens[i]->pValue[0] == '#') {
                if (pTokens[i]->pValue[1] == '0') {
                    if (pTokens[i]->pValue[2] == 'x') {
                        pSymbols[symIndex]->imm.value = (int)strtol(pTokens[i]->pValue + 3, NULL, 16);
                    } else if (pTokens[i]->pValue[2] == 'b') {
                        pSymbols[symIndex]->imm.value = (int)strtol(pTokens[i]->pValue + 3, NULL, 2);
                    } else {
                        pSymbols[symIndex]->imm.value = atoi(pTokens[i]->pValue + 2);
                    }
                } else {
                    pSymbols[symIndex]->imm.value = atoi(pTokens[i]->pValue + 1);
                }
            } else if (pTokens[i]->pValue[0] == '\'') {
                if (pTokens[i]->pValue[1] == '#') {
                    if (pTokens[i]->pValue[2] == '\'') {
                        pSymbols[symIndex]->imm.value = pTokens[i]->pValue[3];
                    }
                } else {
                    if (pTokens[i]->pValue[1] == '\'') {
                        pSymbols[symIndex]->imm.value = pTokens[i]->pValue[2];
                    }
                }
            }
        } else if (isDirective(pTokens[i])) {
            pSymbols[symIndex]->type = TokenType::DIRECTIVE;
            pSymbols[symIndex]->directive.directive = (U8*)malloc(strlen(pTokens[i]->pValue) + 1);
            strcpy((char*)pSymbols[symIndex]->directive.directive, pTokens[i]->pValue);
            
            // if the directive is .global, then consume the next token as the label
            // if (strcmp(pTokens[i]->pValue, ".global") == 0) {
            //     i++;
            //     pSymbols[symIndex]->type = TokenType::LABEL;
            //     pSymbols[symIndex]->label.label = (U8*)malloc(strlen(pTokens[i]->pValue) + 1);
            //     strcpy((char*)pSymbols[symIndex]->label.label, pTokens[i]->pValue);
            // }
        } else {
            pSymbols[symIndex]->type = TokenType::INSTRUCTION;
            pSymbols[symIndex]->instr.instr = Instr::INVALID;
            pSymbols[symIndex]->instr.instr = getInstr(pTokens[i]->pValue);
  
            if (pSymbols[symIndex]->instr.instr == Instr::INVALID) {
                printf("ERROR: Invalid instruction %s at line %d col %d\n", pTokens[i]->pValue, pTokens[i]->nLine, pTokens[i]->nCol);
                return NULL;
            }
        }

        pSymbols[symIndex]->nLine = pTokens[i]->nLine;
        pSymbols[symIndex]->nCol = pTokens[i]->nCol;
        symIndex++;
    }

    pSymbols[symIndex] = NULL;
    return pSymbols;
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

bool isDirective(Token* pToken) {
    return pToken->pValue[0] == '.';
}

void printSymbol(Symbol* pSymbol) {
    char* instrStr = NULL;

    switch (pSymbol->type) {
    case TokenType::REGISTER:
        printf("REGISTER %d %d %d\n", (int)pSymbol->reg.reg, pSymbol->nLine, pSymbol->nCol);
        break;
    case TokenType::INSTRUCTION:
        instrStr = getInstrStr(pSymbol->instr.instr); // Assign inside case
        if (instrStr != NULL) {
            printf("INSTRUCTION %s %d %d\n", instrStr, pSymbol->nLine, pSymbol->nCol);
        } else {
            printf("INSTRUCTION UNKNOWN(%d) %d %d\n", (int)pSymbol->instr.instr, pSymbol->nLine, pSymbol->nCol);
        }
        break;
    case TokenType::IMMEDIATE:
        printf("IMMEDIATE %d %d %d\n", pSymbol->imm.value, pSymbol->nLine, pSymbol->nCol);
        break;
    case TokenType::LABEL:
        printf("LABEL %s %d %d\n", pSymbol->label.label, pSymbol->nLine, pSymbol->nCol);
        break;
    case TokenType::DIRECTIVE:
        printf("DIRECTIVE %s %d %d\n", pSymbol->directive.directive, pSymbol->nLine, pSymbol->nCol);
        break;
    }
}

void freeSymbols(Symbol** pSymbols) {
    for (int i = 0; pSymbols[i] != NULL; i++) {
        switch (pSymbols[i]->type) {
        case TokenType::REGISTER:
            break;
        case TokenType::INSTRUCTION:
            break;
        case TokenType::IMMEDIATE:
            break;
        case TokenType::LABEL:
            free(pSymbols[i]->label.label);
            break;
        case TokenType::DIRECTIVE:
            free(pSymbols[i]->directive.directive);
            break;
        }

        free(pSymbols[i]);
    }

    free(pSymbols);
}
