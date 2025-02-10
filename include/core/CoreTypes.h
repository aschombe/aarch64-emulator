#ifndef CORETYPES_H
#define CORETYPES_H

#include <stdint.h>

#define NUM_BASE_REGS 32 
#define NUM_SPECIAL_REGS 2
// x0->x31 + pc + nzcv register
#define NUM_REGS       (NUM_BASE_REGS + NUM_SPECIAL_REGS)

#define MEM_TOP        0x100000000
#define MEM_SIZE       0xffff0000
#define EXIT_MAGIC_NUM 0xfdeadl

#define U8  uint8_t 
#define U16 uint16_t 
#define U32 uint32_t 
#define U64 uint64_t 
#define I8  int8_t 
#define I16 int16_t 
#define I32 int32_t 
#define I64 int64_t

#define SZ  size_t

typedef struct {
    U64  uLabelAddress;

    U8   uLabelSize;
    char sLabelName[256];
} LabelPair;

#endif