#include "arm64/CFG.h"
#include "arm64/TypeDecoder.h"
#include <stdio.h>

// Additional info:
// the entry block is the one that contains the _start symbol
CFG* buildCFG(Symbol** pSymbols) {
    // When this is building the CFG:
    // construct new symbols so that the new CFG can own them
    // and free the old symbols
    CFG* pCFG = (CFG*)malloc(sizeof(CFG));
    pCFG->blocks = (CFB**)malloc(sizeof(CFB*) * MAX_BLOCKS);
    pCFG->numBlocks = 0;
    pCFG->entry = 0;
    pCFG->exit = 0;

    // This CFB* will be used to build each block
    CFB* pCFB = (CFB*)malloc(sizeof(CFB));
    pCFB->instructions = (Symbol**)malloc(sizeof(Symbol*) * MAX_INSTRUCTIONS);
    pCFB->numInstructions = 0;
    pCFB->edges = (CFB**)malloc(sizeof(CFB*) * MAX_BLOCKS);
    pCFB->numEdges = 0;
    pCFB->id = 0;

    // This is the current block being built
    CFB* pCurrentBlock = pCFB;

    for (int i = 0; pSymbols[i] != NULL; i++) {
        Symbol* pSymbol = pSymbols[i];
        if (pSymbol->type == SymbolType::LABEL) {
            // If the current block is not empty, add it to the CFG
            if (pCurrentBlock->numInstructions > 0) {
                pCFG->blocks[pCFG->numBlocks] = pCurrentBlock;
                pCFG->numBlocks++;
                pCurrentBlock = (CFB*)malloc(sizeof(CFB));
                pCurrentBlock->instructions = (Symbol**)malloc(sizeof(Symbol*) * MAX_INSTRUCTIONS);
                pCurrentBlock->numInstructions = 0;
                pCurrentBlock->edges = (CFB**)malloc(sizeof(CFB*) * MAX_BLOCKS);
                pCurrentBlock->numEdges = 0;
                pCurrentBlock->id = pCFG->numBlocks;
            }
            // Add the label to the current block
            pCurrentBlock->instructions[pCurrentBlock->numInstructions] = pSymbol;
            pCurrentBlock->numInstructions++;
        } else if (isBranch(pSymbol)) {
            // Add the branch instruction to the current block
            pCurrentBlock->instructions[pCurrentBlock->numInstructions] = pSymbol;
            pCurrentBlock->numInstructions++;
            // Add the current block to the CFG
            pCFG->blocks[pCFG->numBlocks] = pCurrentBlock;
            pCFG->numBlocks++;
            // Create a new block for the branch target
            pCurrentBlock = (CFB*)malloc(sizeof(CFB));
            pCurrentBlock->instructions = (Symbol**)malloc(sizeof(Symbol*) * MAX_INSTRUCTIONS);
            pCurrentBlock->numInstructions = 0;
            pCurrentBlock->edges = (CFB**)malloc(sizeof(CFB*) * MAX_BLOCKS);
            pCurrentBlock->numEdges = 0;
            pCurrentBlock->id = pCFG->numBlocks;
        } else {
            // Add the instruction to the current block
            pCurrentBlock->instructions[pCurrentBlock->numInstructions] = pSymbol;
            pCurrentBlock->numInstructions++;
        }

        // If the current symbol is the entry point, set the entry block
        if (pSymbol->type == SymbolType::LABEL && (char*)pSymbol->label.label == "_start") {
            pCFG->entry = pCFG->numBlocks;
        }
    }

    // Add the last block to the CFG
    pCFG->blocks[pCFG->numBlocks] = pCurrentBlock;
    pCFG->numBlocks++;

    return pCFG;
} 

void printCFB(CFB* pCFB) {
    printf("Block %d\n", pCFB->id);
    for (int i = 0; i < pCFB->numInstructions; i++) {
        printSymbol(pCFB->instructions[i]);
    }
    printf("Edges: ");
    for (int i = 0; i < pCFB->numEdges; i++) {
        printf("%d ", pCFB->edges[i]->id);
    }
    printf("Instructions: %d\n", pCFB->numInstructions);
    for (int i = 0; i < pCFB->numInstructions; i++) {
        printSymbol(pCFB->instructions[i]);
    }
    printf("\n");
}

void printCFG(CFG* pCFG) {
    for (int i = 0; i < pCFG->numBlocks; i++) {
        printCFB(pCFG->blocks[i]);
    }
}

void freeCFG(CFG* pCFG) {
    // for (int i = 0; i < pCFG->numBlocks; i++) {
    //     freeCFB(pCFG->blocks[i]);
    // }
    // free(pCFG->blocks);
    // free(pCFG);
}

void freeCFB(CFB* pCFB) {
    // for (int i = 0; i < pCFB->numInstructions; i++) {
    //     freeSymbol(pCFB->instructions[i]);
    // }
}
