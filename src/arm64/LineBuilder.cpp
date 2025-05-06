#include "arm64/LineBuilder.h"
#include "arm64/TypeDecoder.h"
#include "util/InstructionMap.h"
#include "util/Logging.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// Helper function for cleaning up allocated memory on failure within buildLines
static void cleanupPartialLines(Line **pLines, int numLines,
                                Symbol **currentLineSymbols,
                                int numCurrentSymbols) {
    if (currentLineSymbols) {
        for (int i = 0; i < numCurrentSymbols; ++i) {
            freeSymbol(currentLineSymbols[i]);
        }
        free(currentLineSymbols);
    }

    // Free previously completed lines
    if (pLines) {
        for (int i = 0; i < numLines; ++i) {
            if (pLines[i]) {
                if (pLines[i]->symbols) {
                    for (int j = 0; j < pLines[i]->numSymbols; ++j) {
                        freeSymbol(pLines[i]->symbols[j]);
                    }
                    free(pLines[i]->symbols);
                }
                free(pLines[i]);
            }
        }
        free(pLines);
    }
}

// Function to build an array of Lines from a flat array of Symbols
Line **buildLines(Symbol **pSymbols) {
    if (!pSymbols) {
        dologm(ERROR, "buildLines received NULL symbols array.\n");
        return NULL;
    }

    unsigned long initial_lines_capacity = 10;
    int current_lines_count = 0;
    Line **pLines = (Line **)malloc(sizeof(Line *) * initial_lines_capacity);
    if (!pLines) {
        dologm(ERROR, "Failed to allocate memory for lines array.\n");
        return NULL;
    }

    for (int i = 0; i < (int)initial_lines_capacity; ++i)
        pLines[i] = NULL;

    unsigned long initial_symbols_capacity = 10;
    Symbol **current_line_symbols =
        (Symbol **)malloc(sizeof(Symbol *) * initial_symbols_capacity);
    if (!current_line_symbols) {
        dologm(ERROR,
               "Failed to allocate memory for current line symbols array.\n");
        free(pLines);
        return NULL;
    }

    for (int i = 0; i < (int)initial_symbols_capacity; ++i)
        current_line_symbols[i] = NULL;

    int current_symbol_count = 0;
    int current_line_number = pSymbols[0] ? pSymbols[0]->nLine : 0;

    for (size_t i = 0; pSymbols[i] != NULL; ++i) {
        Symbol *current_symbol = pSymbols[i];

        if (current_symbol->type == SymbolType::NEWLINE) {

            if (current_lines_count >= (int)initial_lines_capacity) {
                initial_lines_capacity *= 2;
                Line **temp_lines = (Line **)realloc(
                    pLines, sizeof(Line *) * initial_lines_capacity);
                if (!temp_lines) {
                    dologm(ERROR,
                           "Failed to reallocate memory for lines array.\n");
                    cleanupPartialLines(pLines, current_lines_count,
                                        current_line_symbols,
                                        current_symbol_count);
                    return NULL;
                }
                pLines = temp_lines;
                for (int j = current_lines_count;
                     j < (int)initial_lines_capacity; ++j)
                    pLines[j] = NULL;
            }

            Line *newLine = (Line *)malloc(sizeof(Line));
            if (!newLine) {
                dologm(ERROR, "Failed to allocate memory for a Line struct.\n");
                cleanupPartialLines(pLines, current_lines_count,
                                    current_line_symbols, current_symbol_count);
                return NULL;
            }

            newLine->symbols = NULL;
            newLine->numSymbols = 0;
            newLine->lineNumber = 0;

            newLine->symbols = current_line_symbols;
            newLine->numSymbols = current_symbol_count;
            newLine->lineNumber = current_line_number;

            pLines[current_lines_count++] = newLine;

            initial_symbols_capacity = 10;
            current_line_symbols =
                (Symbol **)malloc(sizeof(Symbol *) * initial_symbols_capacity);
            if (!current_line_symbols) {
                dologm(
                    ERROR,
                    "Failed to allocate memory for next line symbols array.\n");
                cleanupPartialLines(pLines, current_lines_count, NULL, 0);
                return NULL;
            }
            for (int j = 0; j < (int)initial_symbols_capacity; ++j)
                current_line_symbols[j] = NULL;

            current_symbol_count = 0;
            current_line_number = current_symbol->nLine + 1;

        } else {
            if (current_symbol_count >= (int)initial_symbols_capacity) {
                initial_symbols_capacity *= 2;
                Symbol **temp_symbols = (Symbol **)realloc(
                    current_line_symbols,
                    sizeof(Symbol *) * initial_symbols_capacity);
                if (!temp_symbols) {
                    dologm(ERROR, "Failed to reallocate memory for line "
                                  "symbols array.\n");
                    cleanupPartialLines(pLines, current_lines_count,
                                        current_line_symbols,
                                        current_symbol_count);
                    return NULL;
                }
                current_line_symbols = temp_symbols;
                for (int j = current_symbol_count;
                     j < (int)initial_symbols_capacity; ++j)
                    current_line_symbols[j] = NULL;
            }
            current_line_symbols[current_symbol_count] =
                copySymbol(current_symbol);
            if (!current_line_symbols[current_symbol_count]) {
                dologm(ERROR, "Failed to copy symbol during LineBuilder.\n");
                cleanupPartialLines(pLines, current_lines_count,
                                    current_line_symbols,
                                    current_symbol_count + 1);
                return NULL;
            }
            current_symbol_count++;

            if (current_symbol_count == 1) {
                current_line_number = current_symbol->nLine;
            }
        }
    }

    if (current_symbol_count > 0) {
        if (current_lines_count >= (int)initial_lines_capacity) {
            initial_lines_capacity *= 2;
            Line **temp_lines = (Line **)realloc(
                pLines, sizeof(Line *) * initial_lines_capacity);
            if (!temp_lines) {
                dologm(ERROR, "Failed to reallocate memory for lines array "
                              "(last line).\n");
                cleanupPartialLines(pLines, current_lines_count,
                                    current_line_symbols, current_symbol_count);
                return NULL;
            }
            pLines = temp_lines;
            for (int j = current_lines_count; j < (int)initial_lines_capacity;
                 ++j)
                pLines[j] = NULL;
        }

        Line *newLine = (Line *)malloc(sizeof(Line));
        if (!newLine) {
            dologm(ERROR,
                   "Failed to allocate memory for the last Line struct.\n");
            cleanupPartialLines(pLines, current_lines_count,
                                current_line_symbols, current_symbol_count);
            return NULL;
        }
        newLine->symbols = NULL;
        newLine->numSymbols = 0;
        newLine->lineNumber = 0;

        newLine->symbols = current_line_symbols;
        newLine->numSymbols = current_symbol_count;
        newLine->lineNumber = current_line_number;

        pLines[current_lines_count++] = newLine;
    } else {
        free(current_line_symbols);
    }

    if (current_lines_count >= (int)initial_lines_capacity) {
        initial_lines_capacity++;
        Line **temp_lines =
            (Line **)realloc(pLines, sizeof(Line *) * initial_lines_capacity);
        if (!temp_lines) {
            dologm(ERROR, "Failed to reallocate memory for lines array (NULL "
                          "terminator).\n");
            cleanupPartialLines(pLines, current_lines_count, NULL, 0);
            return NULL;
        }
        pLines = temp_lines;
    }
    pLines[current_lines_count] = NULL;

    return pLines;
}

// Function to free the memory allocated for the array of Lines
void freeLines(Line **pLines) {
    if (!pLines)
        return;

    for (size_t i = 0; pLines[i] != NULL; ++i) {
        if (pLines[i]->symbols) {
            for (int j = 0; j < pLines[i]->numSymbols; ++j) {
                freeSymbol(pLines[i]->symbols[j]);
            }
            free(pLines[i]->symbols);
        }
        free(pLines[i]);
    }
    free(pLines);
}

// Helper function to print a single Line
void printLine(Line *pLine) {
    if (!pLine)
        return;
    printf("Line %d (%d symbols): ", pLine->lineNumber, pLine->numSymbols);
    for (int i = 0; i < pLine->numSymbols; ++i) {
        if (pLine->symbols[i]) {
            switch (pLine->symbols[i]->type) {
            case SymbolType::REGISTER:
                printf("REG('%s') ", getRegStr(pLine->symbols[i]->reg.reg));
                break;
            case SymbolType::INSTRUCTION:
                printf("INSTR('%s') ",
                       getInstrStr(pLine->symbols[i]->instr.instr));
                break;
            case SymbolType::IMMEDIATE:
                printf("IMM(%d) ", pLine->symbols[i]->imm.value);
                break;
            case SymbolType::LABEL:
                printf("LABEL('%s') ", pLine->symbols[i]->label.label);
                break;
            case SymbolType::DIRECTIVE:
                printf("DIR('%s') ", pLine->symbols[i]->directive.directive);
                break;
            default:
                printf("UNKNOWN ");
                break;
            }
        } else {
            printf("NULL_SYMBOL ");
        }
    }
    printf("\n");
}

void printLines(Line **pLines) {
    if (!pLines)
        return;
    for (size_t i = 0; pLines[i] != NULL; ++i) {
        printLine(pLines[i]);
    }
}
