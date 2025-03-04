#include "arm64/Optimizer.h"
#include "arm64/TypeDecoder.h"
#include <stdlib.h>

// https://gcc.gnu.org/onlinedocs/gcc/Optimize-Options.html

// Possible optimizations:
// Strength reduction
// Branch optimization (eliminate ex: b. label, .label:)
Symbol** Optimize(Symbol** pSymbols, int level) {
    switch (level) {
        case 1:
            return O1(pSymbols);
        case 2:
            return O2(pSymbols);
        case 3:
            return O3(pSymbols);
        default:
            return pSymbols;
    }
}

Symbol** O1(Symbol** pSymbols) {
    Symbol** pOptimized = (Symbol**)malloc(1 * sizeof(Symbol*));
    if (pOptimized == NULL) {
        return NULL;
    }

    pOptimized = pSymbols;

    // Do:
    // -fcombine-stack-adjustments
    // -fcompare-elim
    // -fcprop-registers
    // -fdce
    // -fdefer-pop
    // -fdse
    // -fforward-propagate
    // Instruction merging

    return pOptimized;
}

Symbol** O2(Symbol** pSymbols) {
    Symbol** pOptimized = (Symbol**)malloc(1 * sizeof(Symbol*));
    if (pOptimized == NULL) {
        return NULL;
    }

    pOptimized = O1(pSymbols);

    // Do:
    // -foptimize-crc
    // -foptimize-sibling-calls
    // -fpeephole2
    // -fgcse (removes redundant code, loads, stores, etc.)
    // Constant folding

    return pOptimized;
}

Symbol** O3(Symbol** pSymbols) {
    Symbol** pOptimized = (Symbol**)malloc(1 * sizeof(Symbol*));
    if (pOptimized == NULL) {
        return NULL;
    }

    pOptimized = O2(pSymbols);

    // Do:
    // -floop-unroll-and-jam
    // -fpeel-loops
    // -fpredictive-commoning
    // -fsplit-loops
    // -fsplit-paths
    // -ftree-loop-distribution
    // -funswitch-loops
    // -fversion-loops-for-strides

    return pOptimized;
}
