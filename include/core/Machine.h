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
        m_aLayout = (label_pair_t*) malloc(sizeof(label_pair_t) * USHRT_MAX);
        m_aMemory = (uint8_t*) malloc(sizeof(uint8_t) * MEM_SIZE);
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

    uint64_t GetMemBottom() { return m_uMemBottom; }
    uint64_t MapMemoryAddress(uint64_t addr);

    uint64_t ReadQuadwordAt(uint64_t address);
    uint32_t ReadWordAt(uint64_t address);
    uint16_t ReadShortAt(uint64_t address);
    uint8_t ReadByteAt(uint64_t address);

    bool WriteQuadwordAt(uint64_t address, uint64_t* data);
    bool WriteWordAt(uint64_t address, uint32_t* data);
    bool WriteShortAt(uint64_t address, uint16_t* data);
    bool WriteByteAt(uint64_t address, uint8_t* data);

    // 4 byte instruction, 8 byte aligned, mask is 0x00000000ffffffff
    uint32_t ReadInstructionAt(uint64_t address);

private:
    uint64_t m_uMemBottom = MEM_TOP - MEM_SIZE;

    label_pair_t* m_pEntryLabel = NULL;
    uint64_t      m_uNumLabels = 0;
    label_pair_t*  m_aLayout;
    char* m_sStdinContents;
    uint8_t*  m_aMemory;
    uint64_t m_aRegisters[NUM_REGS];
};