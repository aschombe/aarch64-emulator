#pragma once

#include "core/CoreTypes.h"
#include "arm64/Parser.h"

enum class Reg {
  X0, X1, X2, X3, X4, X5, X6, X7,
  X8, X9, X10, X11, X12, X13, X14, X15,
  X16, X17, X18, X19, X20, X21, X22, X23,
  X24, X25, X26, X27, X28, X29, X30, X31,
  W0, W1, W2, W3, W4, W5, W6, W7,
  W8, W9, W10, W11, W12, W13, W14, W15,
  W16, W17, W18, W19, W20, W21, W22, W23,
  W24, W25, W26, W27, W28, W29, W30, W31,
  SP, LR, WZR, XZR, NZCV
};

enum class Instr {
  MOV, ADR,
  LDR, LDRB, STR, STRB,
  ADD, SUB, MUL, UDIV, SDIV,
  ADDS, SUBS, MULS, UDIVS, SDIVS,
  AND, ORR, LSL, LSR, ASR, NOT,
  ANDS, ORRS, LSLS, LSRS, ASRS,
  B, BEQ, BNE, BLT, BLE, BGT, BGE,
  CMP, CBZ, CBNZ,
  BL, RET,
  SVC,
  INVALID,
};

enum class SymbolType {
  REGISTER,
  INSTRUCTION,
  IMMEDIATE,
  LABEL,
  DIRECTIVE,
};

struct Register {
  Reg reg;
};

struct Instruction {
  Instr instr;
};

// Immediates are of the following formats:
/*
(-)n
#(-)n
#0xn
#0bn
'c'
#'c'
*/
struct Immediate {
  int value;
};

struct Label {
  U8* label;
};

struct Directive {
  U8* directive;
};

struct Symbol {
  SymbolType type;
  union {
    Register reg;
    Instruction instr;
    Immediate imm;
    Label label;
    Directive directive;
  };
  int nLine;
  int nCol;
};

Symbol** typeDecode(Token** pTokens);

bool isLabel(Token* pToken);

bool isRegister(Token* pToken);

bool isImmediate(Token* pToken);

bool isDirective(Token* pToken);

bool isBranch(Symbol* pSymbol);

void printSymbols(Symbol** pSymbols);

void printSymbol(Symbol* pSymbol);

void freeSymbols(Symbol** pSymbols);

void freeSymbol(Symbol* pSymbol);
