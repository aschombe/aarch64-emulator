#include <stdio.h>

#include "core/Machine.h"
#include "core/Error.h"
#include "util/Logging.h"

int main(int argc, char* argv[]) {
    CEmulatorErrorHandler err;
    CMachine mach;
    uint64_t test = 0x12345678;
    uint64_t readData = 0;

    dolog("argc=%d, argv[0]=%s\n", argc, argv[0]);

    mach.WriteQuadwordAt(MEM_TOP - 24, &test);
    dologm(DEBUG, "wrote    0x%016lx\n", test);
    readData = mach.ReadQuadwordAt(MEM_TOP - 24);
    dologm(DEBUG, "got back 0x%016lx\n", readData);

    return 0;
}