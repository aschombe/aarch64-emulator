#include "arm64/Optimizer.h"
#include "arm64/TypeDecoder.h"
#include <stdlib.h>

// Peephole optimization
// Redundant instruction removal
// Constant folding
// Strength reduction
// Dead code elimination (unused registers, labels, etc.)
// Instructoin merging
// Register allocation improvements
// Branch optimization (eliminate ex: b. label, .label:)
// Load/store optimization
// Loop unrolling
Symbol** Optimize(Symbol** pSymbols) {
    Symbol** pOptimized = (Symbol**)malloc(1 * sizeof(Symbol*));
    if (pOptimized == NULL) {
        return NULL;
    }

    return pOptimized;
}
