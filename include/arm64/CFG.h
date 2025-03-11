#pragma once

#include "arm64/TypeDecoder.h"
#include <stdlib.h>

#define MAX_BLOCKS 100
#define MAX_INSTRUCTIONS 100

struct CFB {
    Symbol** instructions;
    int numInstructions;
    CFB** edges;
    int numEdges;
    int id;
};

struct CFG {
    CFB** blocks;
    int numBlocks;
    int entry;
    int exit;
};

CFG* buildCFG(Symbol** pSymbols);
void printCFB(CFB* pCFB);
void printCFG(CFG* pCFG);
void freeCFG(CFG* pCFG);
void freeCFB(CFB* pCFB);
