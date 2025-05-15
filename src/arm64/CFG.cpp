#include "arm64/CFG.h"
#include "arm64/LineBuilder.h"
#include "arm64/TypeDecoder.h"
#include "util/InstructionMap.h"
#include "util/Logging.h"

#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
    const char *label;
    CFB *block;
} LabelBlockMapEntry;

static void cleanupPartialCFG(CFG *pCFG, int numBlocks, CFB *currentBlock,
                              LabelBlockMapEntry *label_map, int map_size) {
    // Free the current, incomplete block if it exists
    if (currentBlock) {
        if (currentBlock->lines)
            free(currentBlock->lines);
        if (currentBlock->edges)
            free(currentBlock->edges);
        free(currentBlock);
    }

    // Free previously completed blocks
    if (pCFG && pCFG->blocks) {
        for (int i = 0; i < numBlocks; ++i) {
            if (pCFG->blocks[i]) {
                if (pCFG->blocks[i]->lines)
                    free(pCFG->blocks[i]->lines);
                if (pCFG->blocks[i]->edges)
                    free(pCFG->blocks[i]->edges);
                free(pCFG->blocks[i]);
            }
        }
        free(pCFG->blocks);
    }

    if (label_map) {
        free(label_map);
    }

    if (pCFG) {
        free(pCFG);
    }
}

static CFB *createCFB(int id, int startLine) {
    CFB *newBlock = (CFB *)malloc(sizeof(CFB));
    if (!newBlock) {
        dologm(ERROR, "Failed to allocate memory for CFB %d.\n", id);
        return NULL;
    }
    newBlock->id = id;
    newBlock->lines = NULL;
    newBlock->numLines = 0;
    newBlock->lines_capacity = 0;

    newBlock->edges = NULL;
    newBlock->numEdges = 0;
    newBlock->edges_capacity = 0;

    newBlock->startLine = startLine;
    newBlock->endLine = startLine;
    return newBlock;
}

// Does not copy the line, just adds a pointer to it
static bool addLineToCFB(CFB *block, Line *line) {
    if (block->numLines >= block->lines_capacity) {
        int new_capacity =
            (block->lines_capacity == 0) ? 8 : block->lines_capacity * 2;
        Line **temp_lines = (Line **)realloc(
            block->lines, sizeof(Line *) * (unsigned long)new_capacity);
        if (!temp_lines) {
            dologm(ERROR, "Failed to reallocate memory for lines in CFB %d.\n",
                   block->id);
            return false;
        }
        block->lines = temp_lines;
        block->lines_capacity = new_capacity;
    }

    block->lines[block->numLines++] = line;
    block->endLine = line->lineNumber;

    return true;
}

// Helper function to add an edge from one CFB to another
static bool addEdgeToCFB(CFB *source_block, CFB *target_block) {
    if (!source_block || !target_block) {
        dologm(ERROR, "Attempted to add edge with NULL block pointer.\n");
        return false;
    }

    // Check for duplicate edge to the same target block (optional)
    for (int i = 0; i < source_block->numEdges; ++i) {
        if (source_block->edges[i] == target_block) {
            // Edge already exists, skip
            return true;
        }
    }

    if (source_block->numEdges >= source_block->edges_capacity) {
        int new_capacity = (source_block->edges_capacity == 0)
                               ? 2
                               : source_block->edges_capacity * 2;
        CFB **temp_edges = (CFB **)realloc(
            source_block->edges, sizeof(CFB *) * (unsigned long)new_capacity);
        if (!temp_edges) {
            dologm(ERROR, "Failed to reallocate memory for edges in CFB %d.\n",
                   source_block->id);
            return false;
        }
        source_block->edges = temp_edges;
        source_block->edges_capacity = new_capacity;
    }

    source_block->edges[source_block->numEdges++] = target_block;
    return true;
}

// Helper function to add a label to block mapping
static bool addLabelBlockMapping(LabelBlockMapEntry **map, int *map_size,
                                 int *map_capacity, const char *label,
                                 CFB *block) {
    if (!label || !block) {
        dologm(ERROR, "Attempted to add NULL label or block to map.\n");
        return false;
    }

    // Check for duplicate label definitions (Error in assembly)
    for (int i = 0; i < *map_size; ++i) {
        if (strcmp(map[0][i].label, label) == 0) {
            dologm(ERROR, "Duplicate label definition: %s.\n", label);
            return false;
        }
    }

    if (*map_size >= *map_capacity) {
        *map_capacity = (*map_capacity == 0) ? 16 : (*map_capacity) * 2;
        LabelBlockMapEntry *temp_map = (LabelBlockMapEntry *)realloc(
            *map, sizeof(LabelBlockMapEntry) * (unsigned long)(*map_capacity));
        if (!temp_map) {
            dologm(ERROR, "Failed to reallocate memory for label map.\n");
            return false;
        }
        *map = temp_map;
    }

    map[0][*map_size].label = label;
    map[0][*map_size].block = block;
    (*map_size)++;

    return true;
}

// Helper function to look up a CFB by label name
static CFB *lookupBlockByLabel(LabelBlockMapEntry *map, int map_size,
                               const char *label) {
    if (!map || !label)
        return NULL;

    // Currently linear search, maybe switch to hash map for better performance
    for (int i = 0; i < map_size; ++i) {
        if (map[i].label && strcmp(map[i].label, label) == 0) {
            return map[i].block;
        }
    }
    return NULL; // Label not found
}

// Helper function to get the last significant symbol in a block (last
// instruction or directive)
static Symbol *getLastSignificantSymbol(CFB *block) {
    if (!block || block->numLines == 0)
        return NULL;

    for (int i = block->numLines - 1; i >= 0; --i) {
        Line *line = block->lines[i];
        if (!line || line->numSymbols == 0)
            continue;
        for (int j = line->numSymbols - 1; j >= 0; --j) {
            Symbol *symbol = line->symbols[j];
            // Consider instructions and directives significant for control flow
            // analysis
            if (symbol->type == SymbolType::INSTRUCTION ||
                symbol->type == SymbolType::DIRECTIVE) {
                return symbol;
            }
        }
    }
    return NULL; // No significant symbol found
}

// Helper function to get the branch target label symbol from a branch
// instruction Assumes the target is the very next symbol in the same line after
// the instruction.
static Symbol *getBranchTargetSymbol(Symbol *branch_instruction_symbol,
                                     Line *containing_line) {
    if (!branch_instruction_symbol || !containing_line)
        return NULL;

    for (int i = 0; i < containing_line->numSymbols; ++i) {
        if (containing_line->symbols[i] == branch_instruction_symbol) {
            if (i + 1 < containing_line->numSymbols) {
                Symbol *next_symbol = containing_line->symbols[i + 1];
                // Branch targets are typically Labels or sometimes Immediates
                // (for address literals)
                if (next_symbol->type == SymbolType::LABEL ||
                    next_symbol->type == SymbolType::IMMEDIATE) {
                    return next_symbol;
                }
            }
            break; // Found the instruction but no valid target symbol follows
        }
    }
    return NULL; // Instruction not found in line or no valid target follows
}

// Function to build a CFG from the array of Lines
CFG *buildCFG(Line **pLines) {
    if (!pLines) {
        dologm(ERROR, "buildCFG received NULL lines array.\n");
        return NULL;
    }

    CFG *pCFG = (CFG *)malloc(sizeof(CFG));
    if (!pCFG) {
        dologm(ERROR, "Failed to allocate memory for CFG.\n");
        return NULL;
    }
    pCFG->blocks = NULL;
    pCFG->numBlocks = 0;
    pCFG->blocks_capacity = 0;
    pCFG->entryBlockId = -1;

    CFB *currentBlock = NULL;
    bool inNewBlock = true;

    // Pass 1: Create Blocks
    for (size_t i = 0; pLines[i] != NULL; ++i) {
        Line *currentLine = pLines[i];

        // Skip empty lines
        if (currentLine->numSymbols == 0) {
            continue;
        }

        // Check if this line starts a new block
        bool startsNewBlock = false;
        if (currentLine->numSymbols > 0 &&
            currentLine->symbols[0]->type == SymbolType::LABEL) {
            if (currentBlock != NULL && currentBlock->numLines > 0) {
                startsNewBlock = true;
            }
        }

        // TODO: Refine block splitting: A new block also starts immediately
        // after an unconditional branch or return instruction. This requires
        // looking back at the last instruction of the previous block. For
        // simplicity for now, blocks start only at labels or the beginning.

        if (inNewBlock || startsNewBlock) {
            if (currentBlock != NULL) {
                if (pCFG->numBlocks >= pCFG->blocks_capacity) {
                    pCFG->blocks_capacity = (pCFG->blocks_capacity == 0)
                                                ? 8
                                                : pCFG->blocks_capacity * 2;
                    CFB **temp_blocks = (CFB **)realloc(
                        pCFG->blocks,
                        sizeof(CFB *) * (unsigned long)pCFG->blocks_capacity);
                    if (!temp_blocks) {
                        dologm(ERROR, "Failed to reallocate memory for CFG "
                                      "blocks array.\n");
                        cleanupPartialCFG(pCFG, pCFG->numBlocks, currentBlock,
                                          NULL, 0);
                        return NULL;
                    }
                    pCFG->blocks = temp_blocks;
                    for (int j = pCFG->numBlocks; j < pCFG->blocks_capacity;
                         ++j)
                        pCFG->blocks[j] = NULL;
                }
                pCFG->blocks[pCFG->numBlocks++] = currentBlock;
            }

            currentBlock = createCFB(pCFG->numBlocks, currentLine->lineNumber);
            if (!currentBlock) {
                cleanupPartialCFG(pCFG, pCFG->numBlocks, NULL, NULL, 0);
                return NULL;
            }

            inNewBlock = false;
        }

        if (!addLineToCFB(currentBlock, currentLine)) {
            cleanupPartialCFG(pCFG, pCFG->numBlocks, currentBlock, NULL, 0);
            return NULL;
        }
    }

    if (currentBlock != NULL && currentBlock->numLines > 0) {
        if (pCFG->numBlocks >= pCFG->blocks_capacity) {
            pCFG->blocks_capacity =
                (pCFG->blocks_capacity == 0) ? 8 : pCFG->blocks_capacity * 2;
            CFB **temp_blocks = (CFB **)realloc(
                pCFG->blocks,
                sizeof(CFB *) * (unsigned long)pCFG->blocks_capacity);
            if (!temp_blocks) {
                dologm(ERROR, "Failed to reallocate memory for CFG blocks "
                              "array (last block).\n");
                cleanupPartialCFG(pCFG, pCFG->numBlocks, currentBlock, NULL, 0);
                return NULL;
            }
            pCFG->blocks = temp_blocks;
            for (int j = pCFG->numBlocks; j < pCFG->blocks_capacity; ++j)
                pCFG->blocks[j] = NULL;
        }
        pCFG->blocks[pCFG->numBlocks++] = currentBlock;
    } else if (currentBlock != NULL) {
        freeCFB(currentBlock);
    }

    if (pCFG->numBlocks >= pCFG->blocks_capacity) {
        pCFG->blocks_capacity++;
        CFB **temp_blocks = (CFB **)realloc(
            pCFG->blocks, sizeof(CFB *) * (unsigned long)pCFG->blocks_capacity);
        if (!temp_blocks) {
            dologm(ERROR, "Failed to reallocate memory for CFG blocks array "
                          "(final NULL terminator).\n");
            cleanupPartialCFG(pCFG, pCFG->numBlocks, NULL, NULL, 0);
            return NULL;
        }
        pCFG->blocks = temp_blocks;
    }
    pCFG->blocks[pCFG->numBlocks] = NULL;

    // Pass 2: Build Label to Block Map
    LabelBlockMapEntry *label_to_block_map = NULL;
    int map_size = 0;
    int map_capacity = 0;

    for (int i = 0; pCFG->blocks[i] != NULL; ++i) {
        CFB *block = pCFG->blocks[i];
        // Check the first symbol of the first line in the block
        if (block->numLines > 0 && block->lines[0]->numSymbols > 0 &&
            block->lines[0]->symbols[0]->type == SymbolType::LABEL) {
            Symbol *label_symbol = block->lines[0]->symbols[0];
            // Add the label and block to the map
            if (!addLabelBlockMapping(
                    &label_to_block_map, &map_size, &map_capacity,
                    (const char *)label_symbol->label.label, block)) {
                // Error occurred during map building (e.g., duplicate label)
                cleanupPartialCFG(pCFG, pCFG->numBlocks, NULL,
                                  label_to_block_map, map_size);
                return NULL;
            }
        }
    }

    // Pass 3: Build Edges and set Entry Block ID
    for (int i = 0; pCFG->blocks[i] != NULL; ++i) {
        CFB *current_block = pCFG->blocks[i];
        Symbol *last_symbol = getLastSignificantSymbol(current_block);

        bool ends_with_branch = false;
        const char *branch_target_label = NULL;

        if (last_symbol && last_symbol->type == SymbolType::INSTRUCTION) {
            // Find the line containing the last significant symbol to check its
            // context
            Line *containing_line = NULL;
            for (int j = current_block->numLines - 1; j >= 0; --j) {
                Line *line = current_block->lines[j];
                if (line && line->numSymbols > 0) {
                    for (int k = line->numSymbols - 1; k >= 0; --k) {
                        if (line->symbols[k] == last_symbol) {
                            containing_line = line;
                            break;
                        }
                    }
                }
                if (containing_line)
                    break;
            }

            if (isBranch(last_symbol)) {
                ends_with_branch = true;
                // Get the branch target symbol (assuming it's the next symbol
                // in the line)
                Symbol *target_symbol =
                    getBranchTargetSymbol(last_symbol, containing_line);

                if (target_symbol && target_symbol->type == SymbolType::LABEL) {
                    branch_target_label =
                        (const char *)target_symbol->label.label;
                } else if (target_symbol &&
                           target_symbol->type == SymbolType::IMMEDIATE) {
                    // Handle immediate branch targets if necessary (e.g.,
                    // calculated addresses) dologm(WARNING, "Branch target is
                    // an immediate value in block %d, line %d. CFG edge may not
                    // be built.\n", current_block->id, containing_line ?
                    // containing_line->lineNumber : -1);
                    printf("Branch target is an immediate value in block %d, "
                           "line %d. CFG edge may not be built.\n",
                           current_block->id,
                           containing_line ? containing_line->lineNumber : -1);
                    // For now, we can't resolve immediate targets to blocks
                    // easily, so skip edge
                    ends_with_branch = false;
                } else {
                    // dologm(WARNING, "Branch instruction with no discernible
                    // label or immediate target in block %d, line %d.\n",
                    // current_block->id, containing_line ?
                    // containing_line->lineNumber : -1);
                    printf("Branch instruction with no discernible label or "
                           "immediate target in block %d, line %d.\n",
                           current_block->id,
                           containing_line ? containing_line->lineNumber : -1);
                    ends_with_branch = false;
                }
            }
        }

        // Add edges based on control flow
        if (ends_with_branch && branch_target_label) {
            // Branch instruction found with a label target
            CFB *target_block = lookupBlockByLabel(label_to_block_map, map_size,
                                                   branch_target_label);
            if (target_block) {
                if (!addEdgeToCFB(current_block, target_block)) {
                    cleanupPartialCFG(pCFG, pCFG->numBlocks, NULL,
                                      label_to_block_map, map_size);
                    return NULL;
                }
            } else {
                // dologm(WARNING, "Branch target label '%s' not found for block
                // %d. Edge not built.\n", branch_target_label,
                // current_block->id);
                printf("Branch target label '%s' not found for block %d. Edge "
                       "not built.\n",
                       branch_target_label, current_block->id);
                // Handle unresolved labels if necessary (e.g., add edge to a
                // null block or error)
            }

            // If it's a conditional branch, also add a fall-through edge to the
            // next block
            if (last_symbol && last_symbol->type == SymbolType::INSTRUCTION &&
                    (last_symbol->instr.instr >= Instr::BEQ &&
                     last_symbol->instr.instr <= Instr::BGE) ||
                last_symbol->instr.instr == Instr::CBZ ||
                last_symbol->instr.instr == Instr::CBNZ) {

                // Find the next block in sequential order
                CFB *next_block = NULL;
                if (i + 1 < pCFG->numBlocks && pCFG->blocks[i + 1] != NULL) {
                    next_block = pCFG->blocks[i + 1];
                }

                if (next_block) {
                    if (!addEdgeToCFB(current_block, next_block)) {
                        cleanupPartialCFG(pCFG, pCFG->numBlocks, NULL,
                                          label_to_block_map, map_size);
                        return NULL;
                    }
                } else {
                    // dologm(WARNING, "Conditional branch at end of block %d
                    // has no sequential successor block.\n",
                    // current_block->id);
                    printf("Conditional branch at end of block %d has no "
                           "sequential successor block.\n",
                           current_block->id);
                }
            }
        } else {
            // No branch instruction or a non-resolvable branch target at the
            // end Add a fall-through edge to the next block in sequential
            // order, unless it's the very last block or ends with a
            // return/exit.
            if (last_symbol && last_symbol->type == SymbolType::INSTRUCTION &&
                last_symbol->instr.instr == Instr::RET) {
                // Block ends with RETURN, typically no fall-through edge needed
                // in basic CFG
            } else if (last_symbol &&
                       last_symbol->type == SymbolType::DIRECTIVE &&
                       strcmp((char *)last_symbol->directive.directive,
                              ".global") == 0) {
                // Block ends with .global, typically no fall-through in this
                // context
            } else if (i + 1 < pCFG->numBlocks && pCFG->blocks[i + 1] != NULL) {
                // Not the last block or a terminal instruction/directive, add
                // fall-through edge
                CFB *next_block = pCFG->blocks[i + 1];
                if (!addEdgeToCFB(current_block, next_block)) {
                    cleanupPartialCFG(pCFG, pCFG->numBlocks, NULL,
                                      label_to_block_map, map_size);
                    return NULL;
                }
            } else {
                // Last block or ends with a terminal instruction/directive, no
                // fall-through
            }
        }

        // Identify the entry block (_start label) during this pass as we have
        // the map
        if (pCFG->entryBlockId == -1 && current_block->numLines > 0 &&
            current_block->lines[0]->numSymbols > 0 &&
            current_block->lines[0]->symbols[0]->type == SymbolType::LABEL &&
            strcmp((char *)current_block->lines[0]->symbols[0]->label.label,
                   "_start:") == 0) {
            pCFG->entryBlockId = current_block->id;
        }
    }

    if (pCFG->entryBlockId == -1) {
        // dologm(WARNING, "Entry label '_start:' not found in the program.
        // Entry block set to -1.\n");
        printf("Entry label '_start:' not found in the program. Entry block "
               "set to -1.\n");
    }

    // Free the label map after building edges
    if (label_to_block_map)
        free(label_to_block_map);

    return pCFG;
}

// Does NOT free Line objects or edge blocks
void freeCFB(CFB *pCFB) {
    if (!pCFB)
        return;

    if (pCFB->lines) {
        free(pCFB->lines);
    }

    if (pCFB->edges) {
        free(pCFB->edges);
    }

    free(pCFB);
}

// Does NOT free the Line objects
void freeCFG(CFG *pCFG) {
    if (!pCFG)
        return;

    if (pCFG->blocks) {
        for (int i = 0; pCFG->blocks[i] != NULL; ++i) {
            freeCFB(pCFG->blocks[i]);
        }
        free(pCFG->blocks);
    }

    free(pCFG);
}

void printCFB(CFB *pCFB) {
    if (!pCFB)
        return;
    // printf("Block %d (Lines %d-%d, %d lines)\n", pCFB->id, pCFB->startLine,
    //  pCFB->endLine, pCFB->numLines);

    printf("Block %d (ID %d, Lines %d-%d, %d lines)\n", pCFB->id, pCFB->id,
           pCFB->startLine, pCFB->endLine, pCFB->numLines);

    printf("  Lines:\n");
    if (pCFB->lines) {
        for (int i = 0; i < pCFB->numLines; ++i) {
            if (pCFB->lines[i]) {
                printf("    ");
                printLine(pCFB->lines[i]);
            } else {
                printf("    (NULL Line Pointer)\n");
            }
        }
    } else {
        printf("    (No lines)\n");
    }

    printf("  Edges: ");
    if (pCFB->edges) {
        for (int i = 0; i < pCFB->numEdges; ++i) {
            if (pCFB->edges[i]) {
                printf("%d%s", pCFB->edges[i]->id,
                       (i < pCFB->numEdges - 1) ? ", " : "");
            } else {
                printf("NULL%s", (i < pCFB->numEdges - 1) ? ", " : "");
            }
        }
    }
    if (pCFB->numEdges == 0) {
        printf("(None)");
    }
    printf("\n");
}

void printCFG(CFG *pCFG) {
    if (!pCFG)
        return;
    printf("--- Control Flow Graph ---\n");
    printf("Entry Block ID: %d\n", pCFG->entryBlockId);
    printf("Total Blocks: %d\n", pCFG->numBlocks);
    printf("\n");

    if (pCFG->blocks) {
        for (int i = 0; pCFG->blocks[i] != NULL; ++i) {
            printCFB(pCFG->blocks[i]);
            printf("\n");
        }
    } else {
        printf("(No blocks)\n");
    }
    printf("--------------------------\n");
}
