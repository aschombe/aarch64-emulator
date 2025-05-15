#pragma once

#include "arm64/LineBuilder.h"
#include "arm64/TypeDecoder.h"

#include <stdlib.h>

struct CFG;
struct CFB;

typedef struct CFB {
    Line **lines;
    int numLines;
    int lines_capacity;

    struct CFB **edges;
    int numEdges;
    int edges_capacity;

    int id;        // Unique block ID
    int startLine; // The line number of the first line in this block
    int endLine;   // The line number of the last line in this block
} CFB;

typedef struct CFG {
    CFB **blocks;
    int numBlocks;
    int blocks_capacity;

    int entryBlockId; // ID of the entry block (usually contains the _start
                      // label)
} CFG;

CFG *buildCFG(Line **pLines);

// Does NOT free the Line objects, only the CFG and its blocks
void freeCFB(CFB *pCFB);

// Does NOT free the Line objects, only the CFG and its blocks
void freeCFG(CFG *pCFG);

void printCFB(CFB *pCFB);

void printCFG(CFG *pCFG);