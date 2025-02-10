#pragma once

#include <stdint.h>
#include <stdlib.h>
#include <limits.h>

#include "core/CoreTypes.h"

class CMachine;

static CMachine* pMachine = NULL;

struct MachineOpts {
    bool bDebugEnabled;
    bool bPrintMachineState;
};

class CMachine {
public:
    MachineOpts m_sMachineOpts;

    CMachine() {
        m_aLayout = (LabelPair*) malloc(sizeof(LabelPair) * USHRT_MAX);
        m_aMemory = (U8*) malloc(sizeof(U8) * MEM_SIZE);
        m_sStdinContents = (char*) malloc(sizeof(char) * USHRT_MAX);

        pMachine = this;
    }
    ~CMachine() { 
        free(m_aLayout);
        free(m_aMemory);
        free(m_sStdinContents);

        pMachine = NULL;
    }
    static CMachine* GetCurrentMachine() { return pMachine; }

    U64 GetMemBottom() { return m_uMemBottom; }
    U64 MapMemoryAddress(uint64_t addr);

    U64 ReadQuadwordAt(U64 address);
    U32 ReadWordAt(U64 address);
    U16 ReadShortAt(U64 address);
    U8 ReadByteAt(U64 address);

    bool WriteQuadwordAt(U64 address, U64* data);
    bool WriteWordAt(U64 address, U32* data);
    bool WriteShortAt(U64 address, U16* data);
    bool WriteByteAt(U64 address, U8* data);

    U8* ReadToNullTerminator(U64 address);
    bool WriteToNullTerminator(U64 address, U8* data);

    // 4 byte instruction, 8 byte aligned, mask is 0x00000000ffffffff
    U32 ReadInstructionAt(U64 address);

private:
    U64 m_uMemBottom = MEM_TOP - MEM_SIZE;

    LabelPair* m_pEntryLabel = NULL;
    U64        m_uNumLabels = 0;
    LabelPair* m_aLayout;
    char*      m_sStdinContents;
    U8*        m_aMemory;
    U64        m_aRegisters[NUM_REGS];
};