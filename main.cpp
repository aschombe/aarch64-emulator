#include <stdio.h>

#include "core/mach.h"
#include "arm64/dummy.h"

int main(int argc, char* argv[]) {
    helloWorldFunction();
    dummyFunction(NULL);
    return 0;
}