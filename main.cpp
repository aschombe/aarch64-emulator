#include <stdio.h>

#include "core/Machine.h"
#include "core/Error.h"
#include "util/Logging.h"

int main(int argc, char* argv[]) {
    CEmulatorErrorHandler err;
    CMachine mach;
    U64 test = 0x12345678;
    U64 readData = 0;

#ifdef FS_TESTING_ENABLED
    U8  buf[256];
    U8* buf2 = (U8*) malloc(256);
#endif

    dolog("argc=%d, argv[0]=%s\n", argc, argv[0]);

    mach.WriteQuadwordAt(MEM_TOP - 24, &test);
    dologm(DEBUG, "wrote    0x%016lx\n", test);
    readData = mach.ReadQuadwordAt(MEM_TOP - 24);
    dologm(DEBUG, "got back 0x%016lx\n", readData);

#ifdef FS_TESTING_ENABLED
    mach.GetMountedFileSystem()->Read(0, &buf[0], 32); // this line will crash, buf is not owned
    mach.GetMountedFileSystem()->Read(0, buf2, 32);
#endif
    // keep this call, will help us identify unhandled/uncleared errors
    // will primarily be handled in the emulator
    dologm(ERROR, "error handler final state 0x%016x\n", err.GetAllErrors());

#ifdef FS_TESTING_ENABLED
    free(buf2);
#endif

    return 0;
}