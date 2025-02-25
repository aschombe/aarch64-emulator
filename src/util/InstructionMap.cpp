#include "util/InstructionMap.h"

#include <string.h>

const InstructionMap instr_table[] = {
    {"add", Instr::ADD}, {"adds", Instr::ADDS}, {"adr", Instr::ADR},
    {"and", Instr::AND}, {"ands", Instr::ANDS}, {"asr", Instr::ASR},
    {"b", Instr::B}, {"beq", Instr::BEQ}, {"bge", Instr::BGE}, {"bgt", Instr::BGT}, {"bl", Instr::BL},
    {"ble", Instr::BLE}, {"blt", Instr::BLT}, {"bne", Instr::BNE}, {"cbnz", Instr::CBNZ}, {"cbz", Instr::CBZ},
    {"cmp", Instr::CMP}, {"ldr", Instr::LDR}, {"ldrb", Instr::LDRB}, {"lsl", Instr::LSL}, {"lsls", Instr::LSLS},
    {"lsr", Instr::LSR}, {"lsrs", Instr::LSRS}, {"mov", Instr::MOV}, {"mul", Instr::MUL}, {"muls", Instr::MULS},
    {"not", Instr::NOT}, {"orr", Instr::ORR}, {"orrs", Instr::ORRS}, {"ret", Instr::RET}, {"sdiv", Instr::SDIV},
    {"sdivs", Instr::SDIVS}, {"str", Instr::STR}, {"strb", Instr::STRB}, {"sub", Instr::SUB}, {"subs", Instr::SUBS},
    {"svc", Instr::SVC}, {"udiv", Instr::UDIV}, {"udivs", Instr::UDIVS}
};

const int INSTR_TABLE_SIZE = sizeof(instr_table) / sizeof(instr_table[0]);

Instr getInstr(const char* str) {
    for (int i = 0; i < INSTR_TABLE_SIZE; i++) {
        if (strcmp(str, instr_table[i].name) == 0) {
            return instr_table[i].instr;
        }
    }
    return Instr::INVALID;
}

char* getInstrStr(Instr instr) {
    for (int i = 0; i < INSTR_TABLE_SIZE; i++) {
        if (instr == instr_table[i].instr) {
            return (char*)instr_table[i].name;
        }
    }
    return nullptr;
}
