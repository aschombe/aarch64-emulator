#pragma once

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>

class ASTNode {
    public:
    int nLine;
    int nCol;
    
    virtual ~ASTNode() = default;
    virtual char* ToString() = 0;
};

class Label : public ASTNode {
public:
    char* pLabel;

    Label(char* pLabel, int nLine, int nCol) {
        this->pLabel = pLabel;
        this->nLine = nLine;
        this->nCol = nCol;
    }

    char* ToString() override {
        char* pStr = new char[256];
        sprintf(pStr, "Label: %s", pLabel);
        return pStr;
    }
};

class Directive : public ASTNode {
public:
    char* pDirective;
    char* pOperands;

    Directive(char* pDirective, char* pOperands, int nLine, int nCol) {
        this->pDirective = pDirective;
        this->pOperands = pOperands;
        this->nLine = nLine;
        this->nCol = nCol;
    }

    char* ToString() override {
        char* pStr = new char[256];
        sprintf(pStr, "Directive: %s Operands: %s", pDirective, pOperands);
        return pStr;
    }
};

// defines arm instructions
enum class InstructionType {
    // 0 Args
    NOP,
    RET,
    // 1 Arg
    B,
    BL,
    BEQ,
    BNE,
    BGT,
    BLT,
    BGE,
    BLE,
    SVC,
    // 2 Args
    MOV,
    ADR,
    CMP,
    CBZ,
    CBNZ,
    // 3 Args
    ADD,
    ADDS,
    SUB,
    SUBS,
    MUL,
    MULS,
    UDIV,
    UDIVS,
    SDIV,
    SDIVS,
    AND,
    ANDS,
    ORR,
    ORRS,
    LSL,
    LSLS,
    LSR,
    LSRS,
    ASR,
    ASRS,
    // Variable Args
    // Syntax: LDR/STR [reg], [reg, #offset] where offset is optional
    LDR,
    LDRB,
    STR,
    STRB,
};

enum class OperandType {
    REG,
    IMM,
    MEM,
    LABEL,
};

class Operand : public ASTNode {
public:
    OperandType eType;
    char* pValue;

    Operand(OperandType eType, char* pValue, int nLine, int nCol) {
        this->eType = eType;
        this->pValue = pValue;
        this->nLine = nLine;
        this->nCol = nCol;
    }

    char* ToString() override {
        char* pStr = new char[256];
        sprintf(pStr, "Operand: %s", pValue);
        return pStr;
    }
};

class Instruction : public ASTNode {
public:
    InstructionType eType;
    Operand** pOperands;

    Instruction(InstructionType eType, Operand** pOperands, int nLine, int nCol) {
        this->eType = eType;
        this->pOperands = pOperands;
        this->nLine = nLine;
        this->nCol = nCol;
    }

    char* ToString() override {
        char* pStr = new char[256];
        sprintf(pStr, "Instruction: %d", (int)eType);
        return pStr;
    }
};

ASTNode** Parse(char* pFile);

char* readFile(const char* pFile);

char* cleanFileContents(char* pFileContents);

void strLwr(char* pStr);