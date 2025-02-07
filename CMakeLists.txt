cmake_minimum_required(VERSION 3.15...3.31)

project(
    Aarch64Emulator
    VERSION 0.1
    DESCRIPTION "An example project with CMake"
    LANGUAGES CXX)

if(CMAKE_PROJECT_NAME STREQUAL PROJECT_NAME)
    set(CMAKE_CXX_EXTENSIONS OFF)

    set_property(GLOBAL PROPERTY USE_FOLDERS ON)
endif()

# general flags, feel free to remove these if you see fit but I think these are good
set(CMAKE_CXX_FLAGS "-Wall -Wextra -Wconversion -Wdeprecated -Werror=missing-declarations -Werror=redundant-decls")
# change these for specific configurations
set(CMAKE_CXX_FLAGS_DEBUG "-g -Wpedantic -pedantic-errors")
set(CMAKE_CXX_FLAGS_RELEASE "-O3 -DNDEBUG")

add_subdirectory(src)

add_executable(main main.cpp)
target_compile_features(main PRIVATE cxx_std_17)

target_link_libraries(main PRIVATE core arm64)

