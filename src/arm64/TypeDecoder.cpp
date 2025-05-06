#include "arm64/TypeDecoder.h"
#include "arm64/Parser.h"
#include "util/InstructionMap.h"
#include "util/Logging.h"

#include <ctype.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// Helper function to check if a token is the newline token
bool isNewlineToken(Token *pToken) {
    if (!pToken || !pToken->pValue)
        return false;
    return strcmp(pToken->pValue, NEWLINE_TOKEN_VALUE) == 0;
}

// Ends with a colon
bool isLabel(Token *pToken) {
    if (isNewlineToken(pToken))
        return false;
    if (!pToken || !pToken->pValue || strlen(pToken->pValue) == 0)
        return false;
    return pToken->pValue[strlen(pToken->pValue) - 1] == ':';
}

// Starts with 'x' or 'w' followed by 0-31, or specific names (sp, lr, etc.)
bool isRegister(Token *pToken) {
    if (isNewlineToken(pToken))
        return false;
    if (!pToken || !pToken->pValue)
        return false;

    // Check for specific names
    if (strcmp(pToken->pValue, "sp") == 0 ||
        strcmp(pToken->pValue, "lr") == 0 ||
        strcmp(pToken->pValue, "wzr") == 0 ||
        strcmp(pToken->pValue, "xzr") == 0 ||
        strcmp(pToken->pValue, "nzcv") == 0) {
        return true;
    }

    // Check for indexed registers x0-x31, w0-w31
    if ((pToken->pValue[0] == 'x' || pToken->pValue[0] == 'w') &&
        strlen(pToken->pValue) > 1) {
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

// Checks for a number or character literal in supported formats
// (-)n, #(-)n, #0xn, #0bn, 'c', #'c'
bool isImmediate(Token *pToken) {
    if (isNewlineToken(pToken))
        return false;
    if (!pToken || !pToken->pValue || strlen(pToken->pValue) == 0)
        return false;
    const char *valStr = pToken->pValue;

    if (valStr[0] == '#') {
        if (strlen(valStr) == 1)
            return false;        // Just "#" is not an immediate
        if (valStr[1] == '\'') { // #'c'
            return strlen(valStr) >= 4 && valStr[strlen(valStr) - 1] == '\'';
        }
        // Numeric immediate starting with #
        const char *numStr = valStr + 1;
        if (*numStr == '-' && strlen(numStr) > 1)
            numStr++; // Skip '-' after '#'
        if (*numStr == '0') {
            if (strlen(numStr) > 1) {
                if (numStr[1] == 'x' || numStr[1] == 'X') { // #0x
                    if (strlen(numStr) == 2)
                        return false; // Just "#0x"
                    for (size_t i = 2; i < strlen(numStr); i++) {
                        if (!isxdigit(numStr[i]))
                            return false;
                    }
                    return true;
                } else if (numStr[1] == 'b' || numStr[1] == 'B') { // #0b
                    if (strlen(numStr) == 2)
                        return false; // Just "#0b"
                    for (size_t i = 2; i < strlen(numStr); i++) {
                        if (numStr[i] != '0' && numStr[i] != '1')
                            return false;
                    }
                    return true;
                }
            }
        }
        // #n or #-n (decimal)
        for (size_t i = 0; i < strlen(numStr); i++) {
            if (!isdigit(numStr[i]))
                return false;
        }
        return strlen(numStr) > 0;  // Must have digits after # or #-
    } else if (valStr[0] == '\'') { // 'c'
        return strlen(valStr) >= 3 && valStr[strlen(valStr) - 1] == '\'';
    } else if (isdigit(valStr[0]) ||
               (valStr[0] == '-' && strlen(valStr) > 1 &&
                isdigit(valStr[1]))) { // Simple decimal or negative decimal
        for (size_t i = 1; i < strlen(valStr); i++) {
            if (!isdigit(valStr[i]))
                return false;
        }
        return true;
    }

    return false;
}

bool isDirective(Token *pToken) {
    if (isNewlineToken(pToken))
        return false;
    if (!pToken || !pToken->pValue || strlen(pToken->pValue) == 0)
        return false;
    return pToken->pValue[0] == '.';
}

// Returns the string name for a Reg enum value
char *getRegStr(Reg reg) {
    static const char *reg_names[] = {
        "X0",  "X1",  "X2",  "X3",  "X4",  "X5",  "X6",  "X7",  "X8",  "X9",
        "X10", "X11", "X12", "X13", "X14", "X15", "X16", "X17", "X18", "X19",
        "X20", "X21", "X22", "X23", "X24", "X25", "X26", "X27", "X28", "X29",
        "X30", "X31", "W0",  "W1",  "W2",  "W3",  "W4",  "W5",  "W6",  "W7",
        "W8",  "W9",  "W10", "W11", "W12", "W13", "W14", "W15", "W16", "W17",
        "W18", "W19", "W20", "W21", "W22", "W23", "W24", "W25", "W26", "W27",
        "W28", "W29", "W30", "W31", "SP",  "LR",  "WZR", "XZR", "NZCV"};
    // Simple bounds check assuming Reg enum values match array indices
    if ((int)reg >= 0 &&
        (size_t)reg < sizeof(reg_names) / sizeof(reg_names[0])) {
        return (char *)reg_names[(int)reg];
    }
    return (char *)"UNKNOWN_REG";
}

// Decodes an array of tokens into an array of symbols
Symbol **typeDecode(Token **pTokens) {
    size_t numTokens = 0;
    while (pTokens[numTokens] != NULL)
        numTokens++;

    Symbol **pSymbols = (Symbol **)malloc(sizeof(Symbol *) * (numTokens + 1));
    if (!pSymbols)
        return NULL;
    for (size_t k = 0; k <= numTokens; ++k)
        pSymbols[k] = NULL; // Initialize for safer cleanup

    size_t symIndex = 0;

    for (size_t i = 0; i < numTokens; i++) {
        pSymbols[symIndex] = (Symbol *)malloc(sizeof(Symbol));
        if (!pSymbols[symIndex]) {
            for (size_t j = 0; j < symIndex; ++j) {
                freeSymbol(pSymbols[j]);
            }
            free(pSymbols);
            return NULL;
        }

        // Initialize union members to a safe state (e.g., 0 or NULL)
        // This helps avoid reading garbage if a type isn't fully populated
        memset(
            &pSymbols[symIndex]->reg, 0,
            sizeof(
                pSymbols[symIndex]->reg)); // union size is max size of members

        // Check for newline token FIRST
        if (isNewlineToken(pTokens[i])) {
            pSymbols[symIndex]->type = SymbolType::NEWLINE;
        } else if (isLabel(pTokens[i])) {
            pSymbols[symIndex]->type = SymbolType::LABEL;
            pSymbols[symIndex]->label.label =
                (U8 *)strdup(pTokens[i]->pValue); // Use strdup
            if (!pSymbols[symIndex]->label.label) {
                free(pSymbols[symIndex]);
                for (size_t j = 0; j < symIndex; ++j)
                    freeSymbol(pSymbols[j]);
                free(pSymbols);
                return NULL;
            }
        } else if (isRegister(pTokens[i])) {
            pSymbols[symIndex]->type = SymbolType::REGISTER;

            // Use strcmp for specific registers first
            if (strcmp(pTokens[i]->pValue, "sp") == 0)
                pSymbols[symIndex]->reg.reg = Reg::SP;
            else if (strcmp(pTokens[i]->pValue, "lr") == 0)
                pSymbols[symIndex]->reg.reg = Reg::LR;
            else if (strcmp(pTokens[i]->pValue, "wzr") == 0)
                pSymbols[symIndex]->reg.reg = Reg::WZR;
            else if (strcmp(pTokens[i]->pValue, "xzr") == 0)
                pSymbols[symIndex]->reg.reg = Reg::XZR;
            else if (strcmp(pTokens[i]->pValue, "nzcv") == 0)
                pSymbols[symIndex]->reg.reg = Reg::NZCV;
            // Then handle indexed registers w0-w31, x0-x31
            else if (pTokens[i]->pValue[0] == 'w' &&
                     strlen(pTokens[i]->pValue) > 1 &&
                     isdigit(pTokens[i]->pValue[1])) {
                int reg_num = atoi(pTokens[i]->pValue + 1);
                if (reg_num >= 0 && reg_num <= 31) {
                    pSymbols[symIndex]->reg.reg = (Reg)((int)Reg::W0 + reg_num);
                } else {
                    dologm(ERROR,
                           "ERROR: Invalid W register number %s at line %d col "
                           "%d\n",
                           pTokens[i]->pValue, pTokens[i]->nLine,
                           pTokens[i]->nCol);
                    free(pSymbols[symIndex]);
                    for (size_t j = 0; j < symIndex; ++j)
                        freeSymbol(pSymbols[j]);
                    free(pSymbols);
                    return NULL;
                }
            } else if (pTokens[i]->pValue[0] == 'x' &&
                       strlen(pTokens[i]->pValue) > 1 &&
                       isdigit(pTokens[i]->pValue[1])) {
                int reg_num = atoi(pTokens[i]->pValue + 1);
                if (reg_num >= 0 && reg_num <= 31) {
                    pSymbols[symIndex]->reg.reg = (Reg)((int)Reg::X0 + reg_num);
                } else {
                    dologm(ERROR,
                           "ERROR: Invalid X register number %s at line %d col "
                           "%d\n",
                           pTokens[i]->pValue, pTokens[i]->nLine,
                           pTokens[i]->nCol);
                    free(pSymbols[symIndex]);
                    for (size_t j = 0; j < symIndex; ++j)
                        freeSymbol(pSymbols[j]);
                    free(pSymbols);
                    return NULL;
                }
            } else { // Should be caught by isRegister, but as a fallback
                dologm(ERROR,
                       "ERROR: Could not parse register name %s at line %d col "
                       "%d\n",
                       pTokens[i]->pValue, pTokens[i]->nLine, pTokens[i]->nCol);
                free(pSymbols[symIndex]);
                for (size_t j = 0; j < symIndex; ++j)
                    freeSymbol(pSymbols[j]);
                free(pSymbols);
                return NULL;
            }
        } else if (isImmediate(pTokens[i])) {
            pSymbols[symIndex]->type = SymbolType::IMMEDIATE;

            const char *valStr = pTokens[i]->pValue;
            int base = 10;
            bool is_char_literal = false;
            int offset = 0;
            long val = 0; // Use long for strtol result

            if (valStr[0] == '#') {
                offset = 1;
                if (valStr[1] == '-')
                    offset = 2; // Handle # -n
                if (valStr[1] == '0') {
                    if (strlen(valStr) > 2) {
                        if (valStr[2] == 'x' || valStr[2] == 'X') {
                            base = 16;
                            offset = 3;
                        } else if (valStr[2] == 'b' || valStr[2] == 'B') {
                            base = 2;
                            offset = 3;
                        } else {
                            base = 10;
                            offset = 2;
                        } // #0... (decimal)
                    } else { // Just #0
                        base = 10;
                        offset = 1;
                    }
                } else if (valStr[1] == '\'') {
                    is_char_literal = true;
                    offset = 2;
                } // #'c'
                else {
                    base = 10;
                    offset = 1;
                } // #n or #-n
            } else if (valStr[0] == '\'') {
                is_char_literal = true;
                offset = 1;
            } // 'c'
            else if (isdigit(valStr[0]) ||
                     (valStr[0] == '-' && strlen(valStr) > 1 &&
                      isdigit(valStr[1]))) {
                base = 10;
                offset = 0;
            } // Simple decimal or negative decimal
            else {
                dologm(ERROR,
                       "ERROR: Immediate must start with #, ', -, or digit, "
                       "format %s "
                       "at line %d col %d\n",
                       pTokens[i]->pValue, pTokens[i]->nLine, pTokens[i]->nCol);
                free(pSymbols[symIndex]);
                for (size_t j = 0; j < symIndex; ++j)
                    freeSymbol(pSymbols[j]);
                free(pSymbols);
                return NULL;
            }

            if (is_char_literal) {
                if (strlen(valStr) > (size_t)offset &&
                    valStr[strlen(valStr) - 1] == '\'') {
                    pSymbols[symIndex]->imm.value = (int)valStr[offset];
                } else {
                    dologm(ERROR,
                           "ERROR: Malformed character literal %s at line %d "
                           "col %d\n",
                           pTokens[i]->pValue, pTokens[i]->nLine,
                           pTokens[i]->nCol);
                    free(pSymbols[symIndex]);
                    for (size_t j = 0; j < symIndex; ++j)
                        freeSymbol(pSymbols[j]);
                    free(pSymbols);
                    return NULL;
                }
            } else {
                char *endptr;
                val = strtol(valStr + offset, &endptr, base);
                if (*endptr != '\0') {
                    dologm(ERROR,
                           "ERROR: Invalid number format for immediate %s at "
                           "line %d col "
                           "%d\n",
                           pTokens[i]->pValue, pTokens[i]->nLine,
                           pTokens[i]->nCol);
                    free(pSymbols[symIndex]);
                    for (size_t j = 0; j < symIndex; ++j)
                        freeSymbol(pSymbols[j]);
                    free(pSymbols);
                    return NULL;
                }
                pSymbols[symIndex]->imm.value = (int)val;
                // strtol handles the negative sign if it's at the beginning of
                // the string passed to it
            }
        } else if (isDirective(pTokens[i])) {
            pSymbols[symIndex]->type = SymbolType::DIRECTIVE;
            pSymbols[symIndex]->directive.directive =
                (U8 *)strdup(pTokens[i]->pValue); // Use strdup
            if (!pSymbols[symIndex]->directive.directive) {
                free(pSymbols[symIndex]);
                for (size_t j = 0; j < symIndex; ++j) {
                    freeSymbol(pSymbols[j]);
                }
                free(pSymbols);
                return NULL;
            }
            // Check for valid directives after allocation
            const char *directive_value = pTokens[i]->pValue;
            if (!(strcmp(directive_value, ".text") == 0 ||
                  strcmp(directive_value, ".data") == 0 ||
                  strcmp(directive_value, ".bss") == 0 ||
                  strcmp(directive_value, ".extern") == 0 ||
                  strcmp(directive_value, ".global") == 0)) {
                dologm(ERROR, "ERROR: Invalid directive %s at line %d col %d\n",
                       pTokens[i]->pValue, pTokens[i]->nLine, pTokens[i]->nCol);
                free(pSymbols[symIndex]->directive.directive);
                free(pSymbols[symIndex]);
                for (size_t j = 0; j < symIndex; ++j)
                    freeSymbol(pSymbols[j]);
                free(pSymbols);
                return NULL;
            }
        } else {
            // Check if it's an instruction, otherwise assume label
            Instr instr = getInstr(pTokens[i]->pValue);
            if (instr != Instr::INVALID) {
                pSymbols[symIndex]->type = SymbolType::INSTRUCTION;
                pSymbols[symIndex]->instr.instr = instr;
            } else {
                pSymbols[symIndex]->type =
                    SymbolType::LABEL; // Fallback to LABEL
                pSymbols[symIndex]->label.label =
                    (U8 *)strdup(pTokens[i]->pValue); // Use strdup
                if (!pSymbols[symIndex]->label.label) {
                    free(pSymbols[symIndex]);
                    for (size_t j = 0; j < symIndex; ++j) {
                        freeSymbol(pSymbols[j]);
                    }
                    free(pSymbols);
                    return NULL;
                }
            }
        }

        pSymbols[symIndex]->nLine = pTokens[i]->nLine;
        pSymbols[symIndex]->nCol = pTokens[i]->nCol;
        symIndex++;
    }

    // pSymbols[symIndex] is already NULL from initialization
    return pSymbols;
}

// Returns a pointer to a deep copy of a symbol
Symbol *copySymbol(Symbol *original) {
    if (!original)
        return NULL;

    Symbol *copy = (Symbol *)malloc(sizeof(Symbol));
    if (!copy)
        return NULL;

    copy->type = original->type;
    copy->nLine = original->nLine;
    copy->nCol = original->nCol;

    switch (original->type) {
    case SymbolType::REGISTER:
        copy->reg.reg = original->reg.reg;
        break;
    case SymbolType::INSTRUCTION:
        copy->instr.instr = original->instr.instr;
        break;
    case SymbolType::IMMEDIATE:
        copy->imm.value = original->imm.value;
        break;
    case SymbolType::LABEL:
        copy->label.label =
            (U8 *)strdup((char *)original->label.label); // Use strdup
        if (!copy->label.label) {
            free(copy);
            return NULL;
        }
        break;
    case SymbolType::DIRECTIVE:
        copy->directive.directive =
            (U8 *)strdup((char *)original->directive.directive); // Use strdup
        if (!copy->directive.directive) {
            free(copy);
            return NULL;
        }
        break;
    case SymbolType::NEWLINE:
        // No extra data to copy for a newline symbol
        break;
    }

    return copy;
}

bool isBranch(Symbol *pSymbol) {
    if (!pSymbol || pSymbol->type != SymbolType::INSTRUCTION) {
        return false;
    }

    switch (pSymbol->instr.instr) {
    case Instr::B:
    case Instr::BL:
    case Instr::CBZ:
    case Instr::CBNZ:
    case Instr::BEQ:
    case Instr::BNE:
    case Instr::BLT:
    case Instr::BLE:
    case Instr::BGT:
    case Instr::BGE:
        return true;
    default:
        return false;
    }
}

void printSymbols(Symbol **pSymbols) {
    if (!pSymbols)
        return;
    for (size_t i = 0; pSymbols[i] != NULL; i++) {
        printSymbol(pSymbols[i]);
    }
}

void printSymbol(Symbol *pSymbol) {
    if (!pSymbol)
        return;

    switch (pSymbol->type) {
    case SymbolType::REGISTER:
        printf("REGISTER %s %d %d\n", getRegStr(pSymbol->reg.reg),
               pSymbol->nLine, pSymbol->nCol);
        break;
    case SymbolType::INSTRUCTION:
        printf("INSTRUCTION %s %d %d\n", getInstrStr(pSymbol->instr.instr),
               pSymbol->nLine, pSymbol->nCol);
        break;
    case SymbolType::IMMEDIATE:
        printf("IMMEDIATE %d %d %d\n", pSymbol->imm.value, pSymbol->nLine,
               pSymbol->nCol);
        break;
    case SymbolType::LABEL:
        printf("LABEL %s %d %d\n", pSymbol->label.label, pSymbol->nLine,
               pSymbol->nCol);
        break;
    case SymbolType::DIRECTIVE:
        printf("DIRECTIVE %s %d %d\n", pSymbol->directive.directive,
               pSymbol->nLine, pSymbol->nCol);
        break;
    case SymbolType::NEWLINE:
        printf("NEWLINE %d %d\n", pSymbol->nLine, pSymbol->nCol);
        break;
    }
}

void freeSymbols(Symbol **pSymbols) {
    if (!pSymbols)
        return;

    for (size_t i = 0; pSymbols[i] != NULL; i++) {
        freeSymbol(pSymbols[i]);
    }
    free(pSymbols);
}

void freeSymbol(Symbol *pSymbol) {
    if (!pSymbol)
        return;

    switch (pSymbol->type) {
    case SymbolType::REGISTER:
    case SymbolType::INSTRUCTION:
    case SymbolType::IMMEDIATE:
    case SymbolType::NEWLINE:
        break; // No dynamically allocated memory in the union for these types
    case SymbolType::LABEL:
        free(pSymbol->label.label);
        break;
    case SymbolType::DIRECTIVE:
        free(pSymbol->directive.directive);
        break;
    }

    free(pSymbol); // Free the Symbol struct itself
}
