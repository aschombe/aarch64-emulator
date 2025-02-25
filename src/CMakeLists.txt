# add headers here as they are created, add them to the relevant libraries
set(CORE_HEADER_LIST
    "${Aarch64Emulator_SOURCE_DIR}/include/core/Machine.h"
    "${Aarch64Emulator_SOURCE_DIR}/include/core/CoreTypes.h"
    "${Aarch64Emulator_SOURCE_DIR}/include/core/Error.h"
    "${Aarch64Emulator_SOURCE_DIR}/include/core/FileSystem.h")

set(CORE_SOURCE_LIST
    "${Aarch64Emulator_SOURCE_DIR}/src/core/Machine.cpp"
    "${Aarch64Emulator_SOURCE_DIR}/src/core/Error.cpp"
    "${Aarch64Emulator_SOURCE_DIR}/src/core/FileSystem.cpp")

set(ARM64_HEADER_LIST
    "${Aarch64Emulator_SOURCE_DIR}/include/arm64/Parser.h"
    "${Aarch64Emulator_SOURCE_DIR}/include/arm64/Optimizer.h"
    "${Aarch64Emulator_SOURCE_DIR}/include/arm64/TypeDecoder.h")

set(ARM64_SOURCE_LIST
    "${Aarch64Emulator_SOURCE_DIR}/src/arm64/Parser.cpp"
    "${Aarch64Emulator_SOURCE_DIR}/src/arm64/Optimizer.cpp"
    "${Aarch64Emulator_SOURCE_DIR}/src/arm64/TypeDecoder.cpp")

set(UTIL_HEADER_LIST
    "${Aarch64Emulator_SOURCE_DIR}/include/util/Logging.h"
    "${Aarch64Emulator_SOURCE_DIR}/include/util/InstructionMap.h")

set(UTIL_SOURCE_LIST
    "${Aarch64Emulator_SOURCE_DIR}/src/util/Logging.cpp"
    "${Aarch64Emulator_SOURCE_DIR}/src/util/InstructionMap.cpp")

set(HEADER_LIST
    ${CORE_HEADER_LIST}
    ${ARM64_HEADER_LIST}
    ${UTIL_HEADER_LIST})

if (FS_TESTING)
    add_definitions(-DFS_TESTING_ENABLED)
endif()

# general flags, feel free to remove these if you see fit but I think these are good
set(CMAKE_CXX_FLAGS "-std=c++20 -Wall -Wextra -Wconversion -Wdeprecated -Werror=missing-declarations -Werror=redundant-decls")
# change these for specific configurations
set(CMAKE_CXX_FLAGS_DEBUG "-g -Wpedantic -pedantic-errors -fsanitize=address -DLOG_MASK=0xf")
set(CMAKE_CXX_FLAGS_RELEASE "-O3 -DNDEBUG -DLOG_MASK=0xc")

add_library(core ${CORE_SOURCE_LIST} ${CORE_HEADER_LIST})
target_include_directories(core PUBLIC ../include)
target_compile_features(core PUBLIC cxx_std_17)

add_library(arm64 ${ARM64_SOURCE_LIST} ${ARM64_HEADER_LIST})
target_include_directories(arm64 PUBLIC ../include)
target_compile_features(arm64 PUBLIC cxx_std_17)

add_library(util ${UTIL_SOURCE_LIST} ${UTIL_HEADER_LIST})
target_include_directories(util PUBLIC ../include)
target_compile_features(util PUBLIC cxx_std_17)

source_group(
    TREE "${PROJECT_SOURCE_DIR}/include"
    PREFIX "Header Files"
    FILES ${HEADER_LIST})
