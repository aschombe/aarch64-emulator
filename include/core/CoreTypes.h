#ifndef CORETYPES_H
#define CORETYPES_H

#include <stdint.h>

#define NUM_BASE_REGS 32 
#define NUM_SPECIAL_REGS 2
// x0->x31 + pc + nzcv register
#define NUM_REGS       (NUM_BASE_REGS + NUM_SPECIAL_REGS)

#define MEM_TOP        0xffffffff
#define MEM_SIZE       0xffff0000
#define EXIT_MAGIC_NUM 0xfdeadl


typedef struct {
    uint64_t uLabelAddress;

    uint8_t  uLabelSize;
    char     sLabelName[256];
} label_pair_t;

#endif