#pragma once

#include "arm64/TypeDecoder.h"

Symbol** Optimize(Symbol** pSymbols, int level);

Symbol** O1(Symbol** pSymbols);
Symbol** O2(Symbol** pSymbols);
Symbol** O3(Symbol** pSymbols);
