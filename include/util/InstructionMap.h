#pragma once

#include "arm64/TypeDecoder.h"

struct InstructionMap {
    const char *name;
    Instr instr;
};

extern const InstructionMap instr_table[];
extern const int INSTR_TABLE_SIZE;

Instr getInstr(const char *str);

char *getInstrStr(Instr instr);
