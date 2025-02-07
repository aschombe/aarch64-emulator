# add headers here as they are created, add them to the relevant libraries
set(CORE_HEADER_LIST
    "${Aarch64Emulator_SOURCE_DIR}/include/core/Machine.h"
    "${Aarch64Emulator_SOURCE_DIR}/include/core/CoreTypes.h"
    "${Aarch64Emulator_SOURCE_DIR}/include/core/Error.h")

set(ARM64_HEADER_LIST
    "${Aarch64Emulator_SOURCE_DIR}/include/arm64/dummy.h")

set(HEADER_LIST
    ${CORE_HEADER_LIST}
    ${ARM64_HEADER_LIST})

# general flags, feel free to remove these if you see fit but I think these are good
set(CMAKE_CXX_FLAGS "-Wall -Wextra -Wconversion -Wdeprecated -Werror=missing-declarations -Werror=redundant-decls")
# change these for specific configurations
set(CMAKE_CXX_FLAGS_DEBUG "-g -Wpedantic -pedantic-errors")
set(CMAKE_CXX_FLAGS_RELEASE "-O3 -DNDEBUG")

add_library(core core/Machine.cpp ${CORE_HEADER_LIST})
target_include_directories(core PUBLIC ../include)
target_compile_features(core PUBLIC cxx_std_17)

add_library(arm64 arm64/dummy.cpp ${ARM64_HEADER_LIST})
target_include_directories(arm64 PUBLIC ../include)
target_compile_features(arm64 PUBLIC cxx_std_17)

source_group(
    TREE "${PROJECT_SOURCE_DIR}/include"
    PREFIX "Header Files"
    FILES ${HEADER_LIST})