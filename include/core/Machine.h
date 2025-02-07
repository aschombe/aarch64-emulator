#ifndef MACH_H
#define MACH_H

#include <stdint.h>
#include <stdlib.h>
#include <limits.h>

#include "core/CoreTypes.h"

uint64_t mapMemoryAddress(uint64_t addr);
class CMachine;

static CMachine* pMachine = NULL;

struct MachineOpts {
    bool bDebugEnabled;
    bool bPrintMachineState;
};

class CMachine {
public:
    MachineOpts m_sMachineOpts;

    CMachine() { pMachine = this; }
    ~CMachine() { pMachine = NULL; }

    static CMachine* GetCurrentMachine() { return pMachine; }

    uint64_t GetMemBottom() { return m_uMemBottom; }

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
    label_pair_t  m_aLayout[USHRT_MAX];

    char m_sStdinContents[USHRT_MAX] = {0};

    uint64_t m_aRegisters[NUM_REGS] = {0};
    uint8_t  m_aMemory[MEM_SIZE] = {0};
};

#endif